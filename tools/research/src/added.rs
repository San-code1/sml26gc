//! Points numbered from $80 (`added_points`), tried in the emulator.
use crate::console::load;
use cart_model::map_routes::{self, Place, Way};
use cart_model::ram_map::{GAME_MODE, MAP_POINT};
use cart_model::{added_points, patch, world, GameDriver};
use gb_core::{Buttons, Gb};
use std::path::Path;

const CURRENT_MAP: u16 = 0xA68B;
const PAD: [Buttons; 4] = [Buttons::RIGHT, Buttons::LEFT, Buttons::UP, Buttons::DOWN];
const BACK: [usize; 4] = [1, 0, 3, 2];
const STEP: [(i32, i32); 4] = [(24, 0), (-24, 0), (0, -24), (0, 24)];

fn walk(g: &mut Gb, b: Buttons) -> u8 {
    for f in 0..300u32 {
        g.run_frame_with(if f < 6 { b } else { Buttons::NONE });
        g.clear_audio();
    }
    g.read(MAP_POINT)
}

/// Enter the level of the point Mario is on, clear it as at a goal and come back to the map:
/// the level played.
fn clear_here(g: &mut Gb) -> Option<u8> {
    let mut level = None;
    for f in 0..900u32 {
        g.run_frame_with(if f % 30 < 4 && level.is_none() { Buttons::START } else { Buttons::NONE });
        g.clear_audio();
        if g.read(GAME_MODE) == 0x0B && level.is_none() {
            level = Some(g.read(cart_model::ram_map::LEVEL_NUMBER));
        }
        if level.is_some() && g.mode() == cart_model::GameMode::Playing {
            break;
        }
    }
    level?;
    for _ in 0..30 {
        g.run_frame_with(Buttons::NONE);
    }
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

/// A copy with point $80 on the map of `from` (the world map when `rep` is $01, else the zone
/// whose number `rep` is), joined to `from` by a way in its first free direction, opening
/// level `level` and shutting the way back until it is cleared. The direction from `from`,
/// or None.
fn with_added(rom: &mut Vec<u8>, from: u8, rep: u8, level: u8) -> Option<usize> {
    if !added_points::install(rom) {
        println!("FAIL the hooks do not fit this copy");
        return None;
    }
    let mut r = map_routes::read(rom);
    let f = from as usize;
    let d = (0..4).find(|&d| r.ways[f][d].is_none())?;
    let p = added_points::FIRST;
    let (dx, dy) = STEP[d];
    let at = r.places[f];
    let world = rep < 0x1E;
    r.places[p as usize] = if world {
        let (x, y) = at.on_world();
        Place::at_world(x + dx, y + dy)
    } else {
        Place { x: (at.x as i32 + dx) as u8, y: (at.y as i32 + dy) as u8, ..at }
    };
    r.ways[f][d] = Some(Way::through(&[(dx, dy)], p));
    r.ways[p as usize][BACK[d]] = Some(Way::through(&[(-dx, -dy)], from));
    if world {
        r.added.push(p);
    }
    if let Err(e) = map_routes::write(rom, &r) {
        println!("FAIL routes: {e}");
        return None;
    }
    let mut levels = world::point_levels(rom);
    levels[p as usize] = level;
    world::write_point_levels(rom, &levels);
    added_points::write(rom, &levels[0x80..], &[added_points::DIR_BITS[BACK[d]]], rep);
    Some(d)
}

fn flags(g: &mut Gb) -> Vec<u8> {
    (0xA848u16..0xA872).map(|a| g.read(a)).collect()
}

/// Walk to an added point $80 on the world map and in the Tree Zone, see its way back shut,
/// play and clear its level, and see Mario stay on it with only its own flag byte cleared
/// and the way back open.
pub fn point_check(rom_path: &Path) {
    let Some(gb) = load(rom_path) else { return };
    let Ok(base) = patch::expand(gb.cart().rom()) else { return };
    let mut ok = true;
    for (name, from, rep, level, map) in [("world", 0x07u8, 0x01u8, 1u8, 0u8), ("Tree Zone", 0x20, 0x20, 2, 2)] {
        let mut rom = base.clone();
        let Some(d) = with_added(&mut rom, from, rep, level) else { return };
        let Ok(mut g) = Gb::new(rom, 44100) else { return };
        if !g.boot_to_map() || !g.show_map_at(from) {
            println!("FAIL could not reach {from:02x}");
            return;
        }
        let on = walk(&mut g, PAD[d]);
        let stuck = walk(&mut g, PAD[BACK[d]]);
        let (mx, my) = (g.read(0xA842), g.read(0xA843));
        let before = flags(&mut g);
        let played = clear_here(&mut g);
        let after = flags(&mut g);
        let (point, shown, own) = (g.read(MAP_POINT), g.read(CURRENT_MAP), g.read(added_points::FLAG_RAM));
        let changed: Vec<String> = before.iter().zip(&after).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| format!("{:02x}:{a:02x}>{b:02x}", i)).collect();
        let home = walk(&mut g, PAD[BACK[d]]);
        let good = on == 0x80 && stuck == 0x80 && played == Some(level) && point == 0x80 && shown == map && own & 0x80 != 0 && changed.is_empty() && home == from;
        ok &= good;
        println!(
            "{name}: walked to {on:02x} Mario ({mx:02x},{my:02x}), shut way back left him on {stuck:02x}, played {played:?} (want {level}), back on {point:02x} map {shown:02x}, its flag {own:02x}, game flags changed {changed:?}, walked back to {home:02x}: {}",
            if good { "ok" } else { "FAIL" }
        );
    }
    ok &= new_zone_case(&base);
    println!("{}", if ok { "pointcheck ok" } else { "pointcheck FAILED" });
}

/// A new zone made from the Tree Zone, entered from world point $07, with a point $80 beside
/// its entry: written over the Tree Zone at the door as the port does, walked to and cleared.
fn new_zone_case(base: &[u8]) -> bool {
    use cart_model::new_zones::{self, ZonePoint};
    let mut rom = base.to_vec();
    if !added_points::install(&mut rom) {
        println!("FAIL the hooks do not fit this copy");
        return false;
    }
    let Some(mut z) = new_zones::based_on(&rom, 0) else { return false };
    z.door = 0x07;
    z.exits[0].1 = cart_model::doors::Leads::World(0x07);
    let p = added_points::FIRST;
    let Some(e) = z.points.iter().position(|q| q.number == z.entry) else { return false };
    let Some(d) = (0..4).find(|&d| z.points[e].ways[d].is_none()) else { return false };
    let (dx, dy) = STEP[d];
    let at = z.points[e].place;
    z.points[e].ways[d] = Some(Way::through(&[(dx, dy)], p));
    let mut ways: [Option<Way>; 4] = Default::default();
    ways[BACK[d]] = Some(Way::through(&[(-dx, -dy)], z.entry));
    let place = Place { x: (at.x as i32 + dx) as u8, y: (at.y as i32 + dy) as u8, ..at };
    z.points.push(ZonePoint { number: p, place, ways, level: 3, flag: p, shut: 0, pass: None });
    let entry = z.entry;
    if new_zones::store(&mut rom, std::slice::from_ref(&z)).is_err() {
        return false;
    }
    let w = match new_zones::writes_for(&rom, std::slice::from_ref(&z), 0) {
        Ok(w) => w,
        Err(e) => {
            println!("new zone: FAIL {e}");
            return false;
        }
    };
    let Ok(mut g) = Gb::new(rom, 44100) else { return false };
    if !g.boot_to_map() || !g.show_map_at(0x07) {
        return false;
    }
    new_zones::apply(g.cart_mut().rom_mut(), &w);
    for f in 0..400u32 {
        g.run_frame_with(if f < 6 { Buttons::A } else { Buttons::NONE });
        g.clear_audio();
    }
    let entered = g.read(MAP_POINT);
    let on = walk(&mut g, PAD[d]);
    let played = clear_here(&mut g);
    let (point, shown, own) = (g.read(MAP_POINT), g.read(CURRENT_MAP), g.read(added_points::FLAG_RAM));
    let home = walk(&mut g, PAD[BACK[d]]);
    let good = entered == entry && on == p && played == Some(3) && point == p && shown == 2 && own & 0x80 != 0 && home == entry;
    println!("new zone: entered on {entered:02x}, walked to {on:02x}, played {played:?} (want 3), back on {point:02x} map {shown:02x}, its flag {own:02x}, walked back to {home:02x}: {}", if good { "ok" } else { "FAIL" });
    good
}
