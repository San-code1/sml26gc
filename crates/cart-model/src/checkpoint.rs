//! Midway bells (checkpoints) and where Mario starts again after ringing one.
//!
//! The game's own (research `checkpointprobe`, `classesprobe`):
//! - The midway bell is object type $17 in every level (its class becomes $87: touching it
//!   puts kind 7 in $A26C and 0:34C9 sets $A2A0 = $FF). The finish bell is $16.
//! - Losing a life after that, the map keeps it in bit 3 of the point's flag (0F:6F1F); the
//!   next start copies the flag to $A27D (0F:41B9) and $A2A0 from its bit 3 (01:557D), and
//!   01:55AB copies 0:377B + 8 * level (Mario's y and x, the camera's y and x) over the
//!   header's start.
//! - While $A2A0 is set every bell shows as rung and cannot be rung (16:47DF; the bell's
//!   object code works on a copy of its slot at $FFD0, so $FFD6/$FFD7 point at its record).
//! - One restart place per level: entries 0-30; entry 31 lies over code (0:3873).
//! - A bell comes to life only in x order within its 256-pixel page of the object list.
//!
//! Several bells per level (an edited copy, `install`). The point flags' bits 1, 2, 4, 5 and
//! 6 are never read or written by the game (`flagreaders`), so bits 4-5 keep which bell (0-3)
//! was rung, numbered by the level's $17 records in list order (COUNT, from a record's state
//! byte address). $A300 (cartridge RAM the game never uses) holds the number meanwhile.
//! - 0:2DDB `and $7F / ld ($A26C),a / ret` becomes `jp TOUCH`: the same, and for a bell its
//!   number goes to $A300.
//! - 16:47DF `ld a,($A2A0) / and a` becomes `call RING / nop`: a bell after the rung one can
//!   still be rung.
//! - 0F:6F1F `set 3,(hl) / call $2934 / ret` becomes `call KEEP / ret`: $A300 also goes in
//!   bits 4-5 of the flag.
//! - 01:55AB (the copy of the restart entry) becomes `call RESTART / jr $55C6`: the number
//!   comes back from $A27D to $A300; bell 0 uses the game's entry, bells 1-3 the table in
//!   bank $21 (the twin of bank 1, which nothing reads), read with the upper bank bit set like
//!   the object loader's stub.
//! The code lives in 0:3D42-0:3DFF and 0:3FC4-0:3FFE, filler nothing runs or reads
//! (`roomprobe`), written in bank $20 too (with the upper bit set it is mapped at $0000).
use crate::level_start::{ground, Start, SPRITE_W};
use crate::levels::{COLS, ROWS};
use crate::objects::Obj;
use crate::patch::{is_expanded, TWIN};
use crate::rom::offset;

/// The midway bell's object type.
pub const BELL: u8 = 0x17;
/// Most bells a level can have (numbers 0-3 in two flag bits).
pub const MAX: usize = 4;
/// Levels with a restart entry (31's lies over code).
pub const LEVELS: usize = 31;
/// The game's restart table (bank 0): 8 bytes per level, like the header's start.
pub const RESTARTS: usize = 0x377B;
const ENTRY: usize = 8;

const ROOM_A: u16 = 0x3D42;
#[cfg(test)]
const ROOM_A_END: u16 = 0x3E00;
const ROOM_B: u16 = 0x3FC4;
#[cfg(test)]
const ROOM_B_END: u16 = 0x3FFF;
/// Bank $21: bells 1-3 of each level, at (level * 3 + bell - 1) * 8.
const TABLE_BANK: usize = 0x21;
const TABLE: u16 = 0x7C00;

/// The game's code at each hook: (bank, address, bytes).
const TOUCH_SITE: (usize, u16, &[u8]) = (0x00, 0x2DDB, &[0xE6, 0x7F, 0xEA, 0x6C, 0xA2, 0xC9]);
const RING_SITE: (usize, u16, &[u8]) = (0x16, 0x47DF, &[0xFA, 0xA0, 0xA2, 0xA7]);
const KEEP_SITE: (usize, u16, &[u8]) = (0x0F, 0x6F1F, &[0xCB, 0xDE, 0xCD, 0x34, 0x29, 0xC9]);
const RESTART_SITE: (usize, u16, &[u8]) = (0x01, 0x55AB, &[0x21, 0x7B, 0x37, 0xFA, 0x69, 0xA2, 0xCB, 0x27, 0xCB, 0x27, 0xCB, 0x27, 0x5F, 0x16, 0x00, 0x19, 0x11, 0x00, 0xA8, 0x06, 0x08, 0x2A, 0x12, 0x13, 0x05, 0x20, 0xFA]);

fn le(a: u16) -> [u8; 2] {
    a.to_le_bytes()
}

/// The routines, placed: (address, code) and the hooks' entry points.
struct Code {
    routines: Vec<(u16, Vec<u8>)>,
    touch: u16,
    ring: u16,
    keep: u16,
    restart: u16,
}

fn code() -> Code {
    // de = a record's state byte address -> a = number of $17 records before it (max 3)
    let count: Vec<u8> = vec![
        0x1B, 0x1B, 0x21, 0x06, 0xAB, 0x0E, 0x00,
        0x7C, 0xFE, 0xAD, 0x30, 0x22, 0x7D, 0xBB, 0x20, 0x04, 0x7C, 0xBA, 0x28, 0x1A,
        0xE5, 0x7D, 0xC6, 0x05, 0x6F, 0x7C, 0xCE, 0x00, 0x67, 0x7E, 0xFE, BELL, 0x20, 0x01, 0x0C, 0xE1,
        0x7D, 0xC6, 0x06, 0x6F, 0x7C, 0xCE, 0x00, 0x67, 0x18, 0xD9,
        0x79, 0xFE, 0x04, 0x38, 0x02, 0x3E, 0x03, 0xC9,
    ];
    let count_at = ROOM_A;
    let c = le(count_at);
    let touch_at = count_at + count.len() as u16;
    let touch = vec![0xE6, 0x7F, 0xEA, 0x6C, 0xA2, 0xFE, 0x07, 0xC0, 0x7D, 0xE6, 0xE0, 0xF6, 0x06, 0x6F, 0x2A, 0x57, 0x5E, 0xCD, c[0], c[1], 0xEA, 0x00, 0xA3, 0xC9];
    let restart_at = touch_at + touch.len() as u16;
    let copy_at = restart_at + 62;
    let (t, k) = (le(TABLE), le(copy_at));
    let restart = vec![
        0xFA, 0x7D, 0xA2, 0xCB, 0x37, 0xE6, 0x03, 0xEA, 0x00, 0xA3,
        0x4F, 0xFA, 0x69, 0xA2, 0x5F, 0x79, 0xA7, 0x20, 0x0D,
        0x7B, 0x87, 0x87, 0x87, 0x5F, 0x16, 0x00, 0x21, 0x7B, 0x37, 0x19, 0x18, 0x1E,
        0x0D, 0x7B, 0x87, 0x83, 0x81, 0x6F, 0x26, 0x00, 0x29, 0x29, 0x29, 0x11, t[0], t[1], 0x19,
        0xF3, 0x3E, 0x01, 0xEA, 0x00, 0x40, 0xCD, k[0], k[1], 0xAF, 0xEA, 0x00, 0x40, 0xFB, 0xC9,
        0x11, 0x00, 0xA8, 0x06, 0x08, 0x2A, 0x12, 0x13, 0x05, 0x20, 0xFA, 0xC9,
    ];
    let keep_at = ROOM_B;
    let keep = vec![0xC5, 0xFA, 0x00, 0xA3, 0xE6, 0x03, 0xCB, 0x37, 0x47, 0x7E, 0xE6, 0xCF, 0xB0, 0xF6, 0x08, 0x77, 0xC1, 0xC3, 0x34, 0x29];
    let ring_at = keep_at + keep.len() as u16;
    // a = 0 (z) when this bell can be rung: none rung yet, or it comes after the rung one
    let ring = vec![
        0xFA, 0xA0, 0xA2, 0xA7, 0xC8, 0xE5, 0xC5,
        0xF0, 0xD6, 0x57, 0xF0, 0xD7, 0x5F, 0xCD, c[0], c[1], 0x47, 0xFA, 0x00, 0xA3, 0xB8,
        0xC1, 0xE1, 0x38, 0x04, 0x3E, 0x01, 0xA7, 0xC9, 0xAF, 0xC9,
    ];
    Code { routines: vec![(count_at, count), (touch_at, touch), (restart_at, restart), (keep_at, keep), (ring_at, ring)], touch: touch_at, ring: ring_at, keep: keep_at, restart: restart_at }
}

/// What each hook site becomes.
fn sites(c: &Code) -> [((usize, u16, &'static [u8]), Vec<u8>); 4] {
    let (t, r, k, s) = (le(c.touch), le(c.ring), le(c.keep), le(c.restart));
    let mut restart = vec![0u8; RESTART_SITE.2.len()];
    restart[..5].copy_from_slice(&[0xCD, s[0], s[1], 0x18, 0x16]);
    [
        (TOUCH_SITE, vec![0xC3, t[0], t[1], 0, 0, 0]),
        (RING_SITE, vec![0xCD, r[0], r[1], 0]),
        (KEEP_SITE, vec![0xCD, k[0], k[1], 0xC9, 0, 0]),
        (RESTART_SITE, restart),
    ]
}

fn site_at((bank, addr, _): (usize, u16, &[u8])) -> usize {
    offset(bank, addr)
}

/// The game's restart entry of `level` (8 bytes, like the header's start), if it has one.
pub fn entry(rom: &[u8], level: usize) -> Option<&[u8]> {
    (level < LEVELS).then(|| &rom[RESTARTS + level * ENTRY..RESTARTS + (level + 1) * ENTRY])
}

fn table_at(level: usize, bell: usize) -> usize {
    offset(TABLE_BANK, TABLE) + (level * 3 + bell - 1) * ENTRY
}

/// Restart entry of bell `bell` (0-3) of `level`: the game's for bell 0, the hook's table
/// for the others (zeros without the hook).
pub fn bell_entry(rom: &[u8], level: usize, bell: usize) -> [u8; ENTRY] {
    let mut e = [0u8; ENTRY];
    if level >= LEVELS || bell >= MAX {
        return e;
    }
    if bell == 0 {
        e.copy_from_slice(&rom[RESTARTS + level * ENTRY..RESTARTS + (level + 1) * ENTRY]);
    } else if installed(rom) {
        e.copy_from_slice(&rom[table_at(level, bell)..table_at(level, bell) + ENTRY]);
    }
    e
}

/// Write bell `bell`'s restart entry (bells 1-3 need the hook, `install`).
pub fn set_bell_entry(rom: &mut [u8], level: usize, bell: usize, bytes: &[u8]) {
    if level >= LEVELS || bell >= MAX {
        return;
    }
    let at = if bell == 0 { RESTARTS + level * ENTRY } else { table_at(level, bell) };
    rom[at..at + ENTRY].copy_from_slice(&bytes[..ENTRY]);
}

/// The level's bells, as indexes into its list in list order (a bell's number is its place
/// here).
pub fn bells(objs: &[Obj]) -> Vec<usize> {
    objs.iter().enumerate().filter(|(_, o)| o.kind == BELL).map(|(i, _)| i).collect()
}

/// Where Mario starts again after ringing `bell`: just left of it, standing on the first
/// ground below it (or two blocks below it when there is none).
pub fn restart_for(map: &[u8], bell: &Obj) -> Start {
    let x = bell.x.saturating_sub(8);
    let cols = [x as usize / 16, (x + SPRITE_W - 1) as usize / 16];
    let row = bell.y as usize / 16;
    let floor = (row + 1..ROWS).find(|&r| cols.iter().any(|&c| c < COLS && ground(map[r * COLS + c]))).unwrap_or(row + 2);
    Start::standing(x, (floor * 16) as u16)
}

/// Put a bell in a level's list where the game finds it: in x order within its 256-pixel
/// page. Returns its index.
pub fn insert_bell(objs: &mut Vec<Obj>, x: u16, y: u16) -> usize {
    let page = x / 256;
    let at = objs.iter().position(|o| o.x / 256 > page || (o.x / 256 == page && o.x > x)).unwrap_or(objs.len());
    objs.insert(at, Obj { x, y, kind: BELL, flag: false });
    at
}

/// The bells of `objs` as `(x, y)`, in list order.
pub fn bell_places(objs: &[Obj]) -> Vec<(u16, u16)> {
    objs.iter().filter(|o| o.kind == BELL).map(|o| (o.x, o.y)).collect()
}

/// Writes that give `level` the restart entries of its bells (bells past the first need the
/// hook).
pub fn writes(level: usize, objs: &[Obj], map: &[u8]) -> Vec<(usize, Vec<u8>)> {
    if level >= LEVELS {
        return Vec::new();
    }
    objs.iter().filter(|o| o.kind == BELL).take(MAX).enumerate().map(|(n, b)| {
        let mut e = vec![0u8; ENTRY];
        restart_for(map, b).write(&mut e);
        (if n == 0 { RESTARTS + level * ENTRY } else { table_at(level, n) }, e)
    }).collect()
}


/// The copy has the several-bells hook.
pub fn installed(rom: &[u8]) -> bool {
    let c = code();
    rom.len() > TWIN && sites(&c).iter().all(|(s, b)| rom[site_at(*s)..site_at(*s) + b.len()] == b[..]) && c.routines.iter().all(|(a, b)| rom[*a as usize..*a as usize + b.len()] == b[..])
}

/// The hook can go in: an expanded copy with the game's bytes at the hook sites, or the hook.
pub fn can_install(rom: &[u8]) -> bool {
    is_expanded(rom) && (installed(rom) || [TOUCH_SITE, RING_SITE, KEEP_SITE, RESTART_SITE].iter().all(|s| rom[site_at(*s)..site_at(*s) + s.2.len()] == *s.2))
}

/// Put the several-bells hook in an expanded copy (bank 0 code also in its twin $20).
pub fn install(rom: &mut [u8]) -> bool {
    if !can_install(rom) {
        return false;
    }
    let c = code();
    for base in [0, TWIN] {
        for (a, b) in &c.routines {
            rom[base + *a as usize..base + *a as usize + b.len()].copy_from_slice(b);
        }
    }
    for (s, b) in sites(&c) {
        let at = site_at(s);
        rom[at..at + b.len()].copy_from_slice(&b);
        if s.0 == 0 {
            rom[TWIN + at..TWIN + at + b.len()].copy_from_slice(&b);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_routines_fit_their_rooms() {
        let c = code();
        for (a, b) in &c.routines {
            let end = *a as usize + b.len();
            assert!((*a >= ROOM_A && end <= ROOM_A_END as usize) || (*a >= ROOM_B && end <= ROOM_B_END as usize), "{a:04x}");
        }
        assert_eq!(c.restart + 62 + 12, c.restart + c.routines[2].1.len() as u16);
        assert!(table_at(LEVELS - 1, 3) + ENTRY <= offset(TABLE_BANK, 0x7FFF) + 1);
    }

    #[test]
    fn hook_sites_keep_their_length() {
        for (s, b) in sites(&code()) {
            assert_eq!(s.2.len(), b.len());
        }
    }

    #[test]
    fn a_bell_goes_in_x_order_within_its_page() {
        let o = |x| Obj { x, y: 100, kind: 1, flag: false };
        let mut objs = vec![o(10), o(200), o(300), o(600)];
        assert_eq!(insert_bell(&mut objs, 250, 50), 2);
        assert_eq!(insert_bell(&mut objs, 5, 50), 0);
        assert_eq!(insert_bell(&mut objs, 900, 50), 6);
    }

    #[test]
    fn the_restart_stands_on_the_ground_below() {
        let mut map = vec![0x60u8; COLS * ROWS];
        for c in 0..COLS {
            map[30 * COLS + c] = 0x10;
        }
        let s = restart_for(&map, &Obj { x: 2152, y: 26 * 16, kind: BELL, flag: false });
        assert_eq!((s.mario_x, s.mario_y + crate::level_start::FEET), (2144, 30 * 16));
    }

    #[test]
    fn bells_are_counted_in_list_order() {
        let o = |x, kind| Obj { x, y: 100, kind, flag: false };
        assert_eq!(bells(&[o(10, 1), o(20, BELL), o(30, 2), o(40, BELL)]), vec![1, 3]);
    }
}
