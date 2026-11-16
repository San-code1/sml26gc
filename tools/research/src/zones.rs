//! New zones as stand-ins for a game zone, tried in the emulator before any product code.
//!
//! `zonetry` builds two copies of the cartridge: A, the edited copy as the editor makes it,
//! and B, A with the Tree Zone changed as a new zone would change it (picture, an extra
//! point in the Tree Zone's numbers with its place, ways, level and cleared flag, the exit and
//! a world point's door). It then plays A, writing B's differing bytes into the running
//! cartridge while Mario is at the door point or inside the zone and putting A's back
//! elsewhere, with the Tree Zone's cleared flags swapped the same way.
use crate::console::load;
use cart_model::doors::{self, Leads};
use cart_model::map_routes::{self, Place, Way, LEFT, RIGHT};
use cart_model::ram_map::{GAME_MODE, LEVEL_NUMBER, MAP_POINT};
use cart_model::{new_zones, patch, zone_maps, GameDriver};
use gb_core::{Buttons, Gb};
use std::path::Path;

/// The world point that becomes the new zone's door, the extra point, its level.
const DOOR: u8 = 0x07;
const EXTRA: u8 = 0x25;
const EXTRA_LEVEL: u8 = 10;
const TREE: usize = 0;
const CURRENT_ZONE: u16 = 0xA68B;

/// The new zone written over its base while on, with the base zone's cleared flags kept
/// apart.
struct Swap {
    writes: Vec<(usize, Vec<u8>)>,
    undo: Vec<(usize, Vec<u8>)>,
    flags: Vec<u8>,
    on: bool,
    /// The new zone's cleared flags while swapped out; the base zone's while swapped in.
    kept: Vec<u8>,
}

impl Swap {
    fn set(&mut self, g: &mut Gb, on: bool) {
        if self.on == on {
            return;
        }
        let rom = g.cart_mut().rom_mut();
        if on {
            self.undo = new_zones::apply(rom, &self.writes);
        } else {
            new_zones::apply(rom, &self.undo);
        }
        let now: Vec<u8> = self.flags.iter().map(|&f| g.read(0xA848 + f as u16)).collect();
        for (&f, &v) in self.flags.iter().zip(&self.kept) {
            g.write(0xA848 + f as u16, v);
        }
        self.kept = now;
        self.on = on;
    }
}

fn frames(g: &mut Gb, n: u32, b: Buttons) {
    for f in 0..n {
        g.run_frame_with(if f < 6 { b } else { Buttons::NONE });
        g.clear_audio();
    }
}

fn state(g: &mut Gb) -> String {
    format!("point {:02x} zone {} mode {:02x} a690 {:02x} a7a0 {:02x}", g.read(MAP_POINT), g.read(CURRENT_ZONE), g.read(GAME_MODE), g.read(0xA690), g.read(0xA7A0))
}

fn picture_shown(g: &mut Gb) -> Vec<u8> {
    (0..zone_maps::W * zone_maps::H).map(|i| g.read(0x9800 + ((i / zone_maps::W) * 32 + i % zone_maps::W) as u16)).collect()
}

pub fn zone_try(rom_path: &Path) {
    let Some(gb) = load(rom_path) else { return };
    let original = gb.cart().rom().to_vec();
    let mut a = patch::expand(&original).expect("expand");
    let routes = map_routes::read(&a);
    map_routes::write(&mut a, &routes).expect("routes");


    // the new zone: the Tree Zone's copy with another picture and two points, $23 (entry,
    // exit, level 1) and the extra one, a pass point of the game made an ordinary point
    let mut z = new_zones::based_on(&a, TREE).expect("Tree Zone");
    println!("based on the Tree Zone: points {:02x?} entry {:02x} exits {:?}", z.points.iter().map(|p| p.number).collect::<Vec<_>>(), z.entry, z.exits);
    for t in z.picture.iter_mut().take(zone_maps::W * 3) {
        *t = pic_fill(*t);
    }
    let pic = z.picture.clone();
    let p23 = z.points.iter().find(|p| p.number == 0x23).cloned().expect("$23");
    let extra = new_zones::ZonePoint { number: EXTRA, place: Place { x: p23.place.x + 24, ..p23.place }, ways: [None, Some(Way { steps: vec![LEFT; 3], to: 0x23 }), None, None], level: EXTRA_LEVEL, flag: 36, shut: 0, pass: None };
    let mut first = p23.clone();
    first.ways = [Some(Way { steps: vec![RIGHT; 3], to: EXTRA }), None, None, None];
    z.points = vec![first, extra];
    z.exits[0].1 = Leads::World(DOOR);
    z.door = DOOR;
    let mut stored = a.clone();
    new_zones::store(&mut stored, std::slice::from_ref(&z)).expect("store");
    println!("stored and read back the same: {}", new_zones::read(&stored) == vec![z.clone()]);
    // ZONETRY_OUT=<file>: keep this copy (for `newzonecheck`)
    if let Some(out) = std::env::var_os("ZONETRY_OUT") {
        let _ = std::fs::write(out, &stored);
    }
    let writes = new_zones::writes_for(&a, std::slice::from_ref(&z), 0).expect("writes");
    println!("runs written: {}  bytes: {}", writes.len(), writes.iter().map(|w| w.1.len()).sum::<usize>());
    let plain = patch::expand(&original).expect("expand");
    let w = new_zones::writes_for(&plain, std::slice::from_ref(&z), 0).expect("writes on a copy whose routes are the game's");
    let mut r = plain.clone();
    let undo = new_zones::apply(&mut r, &w);
    println!("copy with the game's routes: {} bytes written, ways moved while swapped: {}, undone: {}", w.iter().map(|w| w.1.len()).sum::<usize>(), map_routes::moved(&r), { new_zones::apply(&mut r, &undo); r == plain });
    let flags = new_zones::flags(&a, &new_zones::BASES[0]);
    println!("progress flags: {flags:?}");

    let Ok(mut g) = Gb::new(a.clone(), 44100) else { return };
    if !g.boot_to_map() || !g.show_map_at(DOOR) {
        println!("could not reach the map");
        return;
    }
    let mut swap = Swap { writes, undo: Vec::new(), on: false, kept: vec![0; flags.len()], flags };
    println!("at the door: {}", state(&mut g));
    swap.set(&mut g, true);
    frames(&mut g, 400, Buttons::A);
    println!("after A: {}  picture as B: {}", state(&mut g), picture_shown(&mut g) == pic);
    frames(&mut g, 300, Buttons::RIGHT);
    println!("after Right: {}", state(&mut g));
    println!("clear $23's level: {:?}  {}  flag $A849 {:02x}", clear_here(&mut g), state(&mut g), g.read(0xA849));
    frames(&mut g, 300, Buttons::RIGHT);
    println!("after Right: {}", state(&mut g));
    println!("clear the extra point's level: {:?}  {}  flag $A86C {:02x}  picture as B: {}", clear_here(&mut g), state(&mut g), g.read(0xA86C), picture_shown(&mut g) == pic);
    frames(&mut g, 300, Buttons::LEFT);
    println!("after Left: {}", state(&mut g));
    frames(&mut g, 600, Buttons::DOWN);
    println!("after Down (exit): {}", state(&mut g));
    // away from the door: swap out
    swap.set(&mut g, false);
    println!("swapped out: cartridge as A: {}  base flag $A849 {:02x}  new zone's kept {:02x?}", g.cart().rom() == &a[..], g.read(0xA849), swap.kept);
    g.show_map_at(0x02);
    frames(&mut g, 400, Buttons::A);
    let real = zone_maps::read(&a, TREE);
    println!("real Tree Zone from $02: {}  picture as A: {}", state(&mut g), picture_shown(&mut g) == real);
    // straight onto the extra point (no walk after a clear)
    swap.set(&mut g, true);
    g.show_map_at(EXTRA);
    println!("shown at the extra point: {}  picture as B: {}", state(&mut g), picture_shown(&mut g) == pic);
    let mut h = g.clone();
    frames(&mut h, 300, Buttons::LEFT);
    println!("  after Left: {}", state(&mut h));
    println!("  clear its level: {:?}  {}", clear_here(&mut g), state(&mut g));
    swap.set(&mut g, false);
    println!("real Tree Zone, clear $23's level: {:?}  {}", clear_here(&mut g), state(&mut g));
    println!("again: {:?}  {}", clear_here(&mut g), state(&mut g));

    // entering on the extra point: Mario walks onto its place
    z.entry = EXTRA;
    let w = new_zones::writes_for(&a, std::slice::from_ref(&z), 0).expect("writes");
    let Ok(mut g) = Gb::new(a.clone(), 44100) else { return };
    g.boot_to_map();
    g.show_map_at(DOOR);
    new_zones::apply(g.cart_mut().rom_mut(), &w);
    frames(&mut g, 400, Buttons::A);
    println!("entered on the extra point: {}  Mario ({}, {}) place ({}, {})", state(&mut g), g.read(0xA842), g.read(0xA843), z.points[1].place.x, z.points[1].place.y);
    println!("  its level: {:?}", clear_here(&mut g));
}

/// Enter the level of the point Mario is on and clear it as at a goal (mode $09); the
/// level the game picked, once back on the map.
fn clear_here(g: &mut Gb) -> Option<u8> {
    let mut level = None;
    for f in 0..900u32 {
        g.run_frame_with(if f % 30 < 4 && level.is_none() { Buttons::START } else { Buttons::NONE });
        g.clear_audio();
        if g.read(GAME_MODE) == 0x0B && level.is_none() {
            level = Some(g.read(LEVEL_NUMBER));
        }
        if level.is_some() && g.mode() == cart_model::GameMode::Playing {
            break;
        }
    }
    level?;
    frames(g, 30, Buttons::NONE);
    g.write(GAME_MODE, 0x09);
    let mut on_map = 0;
    for f in 0..6000u32 {
        let tap = match f % 240 { 0..=2 => Buttons::A, 120..=122 => Buttons::START, _ => Buttons::NONE };
        let mode = g.read(GAME_MODE);
        g.run_frame_with(if mode != 0x0C && mode != 0x14 { tap } else { Buttons::NONE });
        g.clear_audio();
        on_map = if g.read(GAME_MODE) == 0x0C { on_map + 1 } else { 0 };
        if on_map > 300 {
            break;
        }
    }
    level
}

/// A tile that is plainly different (the picture's first rows are filled with it).
fn pic_fill(t: u8) -> u8 {
    t ^ 0x01
}

/// Mario's place ($A842/$A843) on each point of every zone a door enters, next to the
/// point's place in the routes, to learn how a zone's entry code places Mario.
pub fn zone_places(rom_path: &Path) {
    let Some(gb) = load(rom_path) else { return };
    let routes = map_routes::read(gb.cart().rom());
    for z in doors::INTO {
        let door = doors::read(gb.cart().rom()).into[doors::INTO.iter().position(|d| d.code == z.code).unwrap()][0];
        let mut g = gb.clone();
        if !g.boot_to_map() || !g.show_map_at(door) {
            continue;
        }
        frames(&mut g, 400, Buttons::A);
        let mut seen = Vec::new();
        for dir in [Buttons::NONE, Buttons::RIGHT, Buttons::UP, Buttons::LEFT, Buttons::DOWN, Buttons::RIGHT, Buttons::UP] {
            frames(&mut g, 200, dir);
            let p = g.read(MAP_POINT);
            if g.read(GAME_MODE) == 0x0C && !seen.contains(&p) {
                seen.push(p);
                let pl = routes.places[p as usize];
                println!("zone code {:04x} point {p:02x}: a842 {:3} a843 {:3}  place x {:3} y {:3} cam {} {}", z.code, g.read(0xA842), g.read(0xA843), pl.x, pl.y, pl.cam_x, pl.cam_y);
            }
        }
    }
}

/// Cartridge RAM bytes that change when a level is cleared and Mario is back on the map:
/// where the game keeps a cleared flag besides $A848 + flag (its save slots).
pub fn clear_ram(rom_path: &Path) {
    let Some(mut g) = load(rom_path) else { return };
    if !g.boot_to_map() || !g.show_map_at(0x02) {
        return;
    }
    frames(&mut g, 400, Buttons::A);
    let snap = |g: &mut Gb| (0xA000u16..0xC000).map(|a| g.read(a)).collect::<Vec<u8>>();
    let before = snap(&mut g);
    println!("cleared: {:?}  {}", clear_here(&mut g), state(&mut g));
    let after = snap(&mut g);
    for (i, (x, y)) in before.iter().zip(&after).enumerate() {
        if x != y {
            print!("{:04x}:{x:02x}>{y:02x} ", 0xA000 + i);
        }
    }
    println!();
}

/// Play every new zone of an edited copy: through its door, onto its entry point with its own
/// picture, its entry point's level, then out by each exit (trying each pad direction).
pub fn new_zone_check(rom_path: &Path) {
    let Some(gb) = load(rom_path) else { return };
    let rom = gb.cart().rom().to_vec();
    let zones = new_zones::read(&rom);
    println!("{} new zones", zones.len());
    for (k, z) in zones.iter().enumerate() {
        let name = zone_maps::ZONES[z.base].name;
        if z.door == doors::NONE {
            println!("zone {k} ({name}): no door, not played");
            continue;
        }
        let writes = match new_zones::writes_for(&rom, &zones, k) {
            Ok(w) => w,
            Err(e) => {
                println!("zone {k} ({name}): FAIL {e}");
                continue;
            }
        };
        let mut g = gb.clone();
        if !g.boot_to_map() || !g.show_map_at(z.door) {
            println!("zone {k} ({name}): FAIL could not reach door {:02x}", z.door);
            continue;
        }
        new_zones::apply(g.cart_mut().rom_mut(), &writes);
        frames(&mut g, 400, Buttons::A);
        let entered = g.read(MAP_POINT) == z.entry && picture_shown(&mut g) == z.picture;
        let level = z.points.iter().find(|p| p.number == z.entry).map(|p| p.level).unwrap_or(cart_model::world::NO_LEVEL);
        let played = if level == cart_model::world::NO_LEVEL { "no level".to_string() } else { format!("level {:?}", clear_here(&mut g)) };
        println!("zone {k} ({name}): door {:02x} entered on {:02x}: {}  {played}", z.door, z.entry, if entered { "ok" } else { "FAIL" });
        for &(at, leads) in z.exits.iter().filter(|e| e.0 != doors::NONE) {
            let mut out = None;
            for pad in [Buttons::DOWN, Buttons::UP, Buttons::LEFT, Buttons::RIGHT] {
                let mut h = g.clone();
                h.write(MAP_POINT, at);
                frames(&mut h, 600, pad);
                if h.read(CURRENT_ZONE) == 0 || h.read(MAP_POINT) != at && !cart_model::new_zones::base_of(z.base).is_some_and(|b| b.holds(h.read(MAP_POINT))) {
                    out = Some(h.read(MAP_POINT));
                    break;
                }
            }
            let ok = matches!((leads, out), (Leads::World(p), Some(q)) if p == q) || matches!(leads, Leads::Zone(_)) && out.is_some();
            println!("  exit on {at:02x} to {leads:?}: came out on {out:02x?} {}", if ok { "ok" } else { "FAIL" });
        }
    }
}

/// A new zone with a point on a spare number (`new_zones::SPARE`): walk onto it, clear its
/// level and come back to the new zone's map (the map code's hook), then walk back.
pub fn spare_check(rom_path: &Path) {
    let Some(gb) = load(rom_path) else { return };
    let mut a = patch::expand(gb.cart().rom()).expect("expand");
    let routes = map_routes::read(&a);
    map_routes::write(&mut a, &routes).expect("routes");
    let mut z = new_zones::based_on(&a, TREE).expect("Tree Zone");
    for t in z.picture.iter_mut().take(zone_maps::W * 3) {
        *t = pic_fill(*t);
    }
    let spare = new_zones::SPARE[0];
    let b = new_zones::base_of(TREE).expect("base");
    let flag = *new_zones::flags(&a, b).last().expect("flags");
    let p23 = z.points.iter().position(|p| p.number == 0x23).expect("$23");
    let place = Place { x: z.points[p23].place.x + 24, ..z.points[p23].place };
    z.points[p23].ways[0] = Some(Way { steps: vec![RIGHT; 3], to: spare });
    z.points.push(new_zones::ZonePoint { number: spare, place, ways: [None, Some(Way { steps: vec![LEFT; 3], to: 0x23 }), None, None], level: EXTRA_LEVEL, flag, shut: 0, pass: None });
    z.door = DOOR;
    let writes = new_zones::writes_for(&a, std::slice::from_ref(&z), 0).expect("writes");
    let Ok(mut g) = Gb::new(a.clone(), 44100) else { return };
    if !g.boot_to_map() || !g.show_map_at(DOOR) {
        println!("could not reach the map");
        return;
    }
    let undo = new_zones::apply(g.cart_mut().rom_mut(), &writes);
    frames(&mut g, 400, Buttons::A);
    println!("entered: {}  picture as the new zone: {}", state(&mut g), picture_shown(&mut g) == z.picture);
    println!("cleared $23's level {:?}: {}", clear_here(&mut g), state(&mut g));
    frames(&mut g, 300, Buttons::RIGHT);
    let on = g.read(MAP_POINT) == spare;
    println!("after Right: {}  on the spare point: {}", state(&mut g), if on { "ok" } else { "FAIL" });
    let level = clear_here(&mut g);
    let back = g.read(MAP_POINT) == spare && g.read(CURRENT_ZONE) == 2 && picture_shown(&mut g) == z.picture;
    println!("cleared its level {level:?} (wanted {EXTRA_LEVEL}): {}  flag {flag:02x} = {:02x}  back on the new zone: {}", state(&mut g), g.read(0xA848 + flag as u16), if back { "ok" } else { "FAIL" });
    frames(&mut g, 300, Buttons::LEFT);
    println!("after Left: {}  {}", state(&mut g), if g.read(MAP_POINT) == 0x23 { "ok" } else { "FAIL" });
    new_zones::apply(g.cart_mut().rom_mut(), &undo);
    println!("undone: cartridge as before: {}", g.cart().rom() == &a[..]);

    // a game zone entered on another of its points
    for (zone, entry) in [(TREE, 0x21u8), (2, 0x36)] {
        let mut c = a.clone();
        new_zones::set_entry(&mut c, zone, entry, routes.places[entry as usize]).expect("entry");
        let door = doors::read(&c).into[doors::INTO.iter().position(|d| d.zone == zone).unwrap()][0];
        let Ok(mut g) = Gb::new(c, 44100) else { return };
        g.boot_to_map();
        g.show_map_at(door);
        frames(&mut g, 400, Buttons::A);
        let pl = routes.places[entry as usize];
        println!("{} entered on {entry:02x}: {}  Mario ({}, {}) place ({}, {})  {}", zone_maps::ZONES[zone].name, state(&mut g), g.read(0xA842), g.read(0xA843), pl.x, pl.y, if g.read(MAP_POINT) == entry { "ok" } else { "FAIL" });
    }
}

/// Cartridge RAM the game never writes while it is played (maps, zones, levels, clears,
/// saving): bytes filled with a mark on the map that still hold it at the end, for two marks.
pub fn ram_free(rom_path: &Path) {
    let Some(gb) = load(rom_path) else { return };
    let ranges = [(0xA0F0u16, 0xA200u16), (0xA300, 0xA400), (0xA872, 0xA880), (0xA8A0, 0xA8C0), (0xA900, 0xAA00), (0xAC00, 0xAE00), (0xB000, 0xBA00), (0xBC00, 0xBE00)];
    let mut still: Vec<u16> = ranges.iter().flat_map(|&(a, b)| a..b).collect();
    for mark in [0xA5u8, 0x5A] {
        let mut g = gb.clone();
        if !g.boot_to_map() {
            println!("could not reach the map");
            return;
        }
        for &a in &still {
            g.write(a, mark);
        }
        g.show_map_at(0x28);
        println!("mark {mark:02x}: pumpkin {:?} {}", clear_here(&mut g), state(&mut g));
        g.show_map_at(0x07);
        println!("mark {mark:02x}: world {:?} {}", clear_here(&mut g), state(&mut g));
        g.show_map_at(0x3C);
        frames(&mut g, 300, Buttons::NONE);
        if g.enter_level_anywhere(3) {
            for f in 0..1500u32 {
                g.run_frame_with(match f % 50 { 0..=20 => Buttons::RIGHT, 21..=30 => Buttons::A | Buttons::RIGHT, _ => Buttons::RIGHT | Buttons::B });
                g.clear_audio();
            }
        }
        frames(&mut g, 600, Buttons::NONE);
        still.retain(|&a| g.read(a) == mark);
    }
    let mut runs: Vec<(u16, u16)> = Vec::new();
    for a in still {
        match runs.last_mut() {
            Some(r) if r.1 + 1 == a => r.1 = a,
            _ => runs.push((a, a)),
        }
    }
    for (a, b) in runs.iter().filter(|r| r.1 - r.0 >= 7) {
        println!("{a:04x}-{b:04x} ({} bytes)", b - a + 1);
    }
}
