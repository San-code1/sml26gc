//! Points beyond the numbers the game uses: each map (the world map, a game zone, a new zone)
//! can have `MAX` more, numbered from `FIRST`. No code of the game singles out $80-$FF; the
//! ways and places (`map_routes`, bank $36) are indexed with 16 bits, so they only need room.
//! What else the map code reads by point number or by flag goes through hooks in bank $0F:
//!
//! * the point's level (0F:4218, read at 0F:41A9 and 0F:61E3) and cleared flag (0F:650E, read
//!   at 0F:417D, 0F:433F, 0F:61F2, 0F:6EF6 and 0F:6FD9): an added point's level is in
//!   `LEVELS`, its flag is its own number;
//! * by flag, the flag the walk on after a level looks at (0F:4364, read at 0F:434C: an added
//!   point's own, so he does not walk on), the pad directions it shuts until cleared (0F:6591,
//!   read at 0F:435B and 0F:61FA: `SHUTS`) and the flag byte ($A848 + flag, at 0F:4186,
//!   0F:4352, 0F:6202, 0F:6F01): an added point's is at `FLAG_RAM` + (number - `FIRST`), 96
//!   bytes after the sprite buffer ($A100-$A19F) that the game never touches;
//! * the shut directions are only looked at off the world map (0F:61D7 `ld a,($A68B) / or a`
//!   before `jr z`): `WORLD_SHUT` makes that test not zero for an added point there too;
//! * the map shown is picked from the number (0F:7003), the bonus game is entered from points
//!   $6E and up (0F:420C) and the walk on is for zone points, $1E and up (0F:44BD): those
//!   reads see `REP`, a number of the map shown, instead of an added number.
//!
//! Only one map's added points are in place at a time (the world map's in the copy as
//! written); the port writes the shown map's in (`map_routes` entries, `LEVELS`, `SHUTS`,
//! `REP`: `writes_for_zone`, and a new zone's with the zone) and keeps their flag bytes per
//! map. Each hook is `ld hl,table / bit 7,reg / ret z / ld hl,added table - $80 / ret`.
//! The game zones' added points are kept in bank $24 (the twin of $04, which nothing reads):
//! "ADP1", the count of zones, then each: its `zone_maps` index, its count of points and the
//! points as new zones keep theirs.
use crate::error::CartError;
use crate::new_zones::{self, ZonePoint};
use crate::patch::{is_expanded, TWIN};
use crate::rom::{offset, BANK_SIZE};
use crate::{map_routes, world, zone_maps};

/// The first added point's number.
pub const FIRST: u8 = 0x80;
/// Added points a map can have.
pub const MAX: usize = 40;
/// Where the added points' flag bytes are, in cartridge RAM.
pub const FLAG_RAM: u16 = 0xA1A0;
pub const NO_LEVEL: u8 = 0xFE;
/// The pad bits ($FF80) of right, left, up and down: the directions a point shuts until its
/// level is cleared (0F:61FA) are kept as these.
pub const DIR_BITS: [u8; 4] = [0x10, 0x20, 0x40, 0x80];

const CODE: u16 = 0x7F00;
const REP: u16 = 0x7F5A;
const IDENT: u16 = 0x7F60;
const LEVELS: u16 = IDENT + MAX as u16;
const SHUTS: u16 = LEVELS + MAX as u16;
const WORLD_SHUT: u16 = SHUTS + MAX as u16;
/// 0F:61D7 `ld a,($A68B) / or a`, and the call to `WORLD_SHUT` with a `nop` in its place.
const SHUT_TEST: u16 = 0x61D7;
const SHUT_TEST_BYTES: [u8; 4] = [0xFA, 0x8B, 0xA6, 0xB7];
/// ld a,($A68B) / or a / ret nz / ld a,($A840) / and $80 / ret
const WORLD_SHUT_CODE: [u8; 11] = [0xFA, 0x8B, 0xA6, 0xB7, 0xC0, 0xFA, 0x40, 0xA8, 0xE6, 0x80, 0xC9];

const BIT_A: u8 = 0x7F;
const BIT_C: u8 = 0x79;
const BIT_E: u8 = 0x7B;
const POINT_FLAGS: u16 = 0x650E;
const POINT_LEVELS: u16 = 0x4218;
const NEXT_FLAGS: u16 = 0x4364;
const SHUT: u16 = 0x6591;
const FLAG_BYTES: u16 = 0xA848;

/// The hooks after `REPA`: the table each stands for, the register holding the number, the
/// added table.
const HOOKS: [(u16, u8, u16); 8] = [
    (POINT_FLAGS, BIT_A, IDENT),
    (POINT_FLAGS, BIT_C, IDENT),
    (POINT_LEVELS, BIT_A, LEVELS),
    (NEXT_FLAGS, BIT_E, IDENT),
    (SHUT, BIT_E, SHUTS),
    (SHUT, BIT_A, SHUTS),
    (FLAG_BYTES, BIT_E, FLAG_RAM),
    (FLAG_BYTES, BIT_C, FLAG_RAM),
];
const HOOK_LEN: u16 = 10;

fn hook_at(i: usize) -> u16 {
    CODE + HOOK_LEN * (i as u16 + 1)
}

/// `ld hl,table` sites: (address, hook index).
const SITES: [(u16, usize); 13] = [(0x417D, 0), (0x433F, 0), (0x6EF6, 0), (0x6FD9, 0), (0x61F2, 1), (0x41A9, 2), (0x434C, 3), (0x435B, 4), (0x61FA, 5), (0x4186, 6), (0x4352, 6), (0x6F01, 6), (0x6202, 7)];
/// `ld hl,$4218 / ld a,($A840)` at 0F:61E3, turned round so the number is in a first.
const LEVEL_SITE: u16 = 0x61E3;
/// `ld a,($A840)` sites that see `REP` for an added number (0F:7003 is `MAP_PICK`).
const REP_SITES: [u16; 2] = [0x420C, 0x44BD];
pub const MAP_PICK: u16 = 0x7003;
const READ_POINT: [u8; 3] = [0xFA, 0x40, 0xA8];

fn set(rom: &mut [u8], o: usize, v: u8) {
    rom[o] = v;
    if rom.len() >= 2 * TWIN && o < TWIN {
        rom[o + TWIN] = v;
    }
}

fn call(to: u16) -> [u8; 3] {
    [0xCD, to as u8, (to >> 8) as u8]
}

fn ld_hl(t: u16) -> [u8; 3] {
    [0x21, t as u8, (t >> 8) as u8]
}

/// The call that reads the point number for the map pick and the like.
pub fn read_point() -> [u8; 3] {
    call(CODE)
}

fn code() -> Vec<u8> {
    let mut v = vec![0xFA, 0x40, 0xA8, 0xCB, BIT_A, 0xC8, 0xFA, REP as u8, (REP >> 8) as u8, 0xC9];
    for (table, bit, added) in HOOKS {
        let alt = added.wrapping_sub(FIRST as u16);
        v.extend(ld_hl(table));
        v.extend([0xCB, bit, 0xC8]);
        v.extend(ld_hl(alt));
        v.push(0xC9);
    }
    v
}

/// Are the hooks in this copy?
pub fn installed(rom: &[u8]) -> bool {
    let (t, w) = (offset(0x0F, SHUT_TEST), offset(0x0F, WORLD_SHUT));
    first_hooks_in(rom) && rom[t..t + 4] == shut_test_call() && rom[w..w + WORLD_SHUT_CODE.len()] == WORLD_SHUT_CODE
}

/// The hooks but `WORLD_SHUT` (a copy saved before it was added).
fn first_hooks_in(rom: &[u8]) -> bool {
    let c = offset(0x0F, CODE);
    let s = offset(0x0F, REP_SITES[0]);
    rom.len() > offset(0x0F, 0x7FFF) && rom[c..c + code().len()] == code()[..] && rom[s..s + 3] == read_point()
}

fn shut_test_call() -> [u8; 4] {
    let c = call(WORLD_SHUT);
    [c[0], c[1], c[2], 0x00]
}

/// Can the hooks go in? The map code must be the game's at every site (or already hooked).
pub fn can_install(rom: &[u8]) -> bool {
    if installed(rom) {
        return true;
    }
    let at = |a: u16| offset(0x0F, a);
    if rom.len() <= at(0x7FFF) || rom[at(SHUT_TEST)..at(SHUT_TEST) + 4] != SHUT_TEST_BYTES {
        return false;
    }
    first_hooks_in(rom)
        || SITES.iter().all(|&(a, i)| rom[at(a)..at(a) + 3] == ld_hl(HOOKS[i].0))
        && rom[at(LEVEL_SITE)..at(LEVEL_SITE) + 6] == [0x21, 0x18, 0x42, 0xFA, 0x40, 0xA8]
        && REP_SITES.iter().all(|&a| rom[at(a)..at(a) + 3] == READ_POINT)
}

/// Put the hooks in (the copy's map code as the game's otherwise; nothing added in place).
pub fn install(rom: &mut [u8]) -> bool {
    if installed(rom) {
        return true;
    }
    if !can_install(rom) {
        return false;
    }
    let at = |a: u16| offset(0x0F, a);
    for (k, b) in WORLD_SHUT_CODE.into_iter().enumerate() {
        set(rom, at(WORLD_SHUT) + k, b);
    }
    for (k, b) in shut_test_call().into_iter().enumerate() {
        set(rom, at(SHUT_TEST) + k, b);
    }
    if first_hooks_in(rom) {
        return true;
    }
    for (i, b) in code().into_iter().enumerate() {
        set(rom, at(CODE) + i, b);
    }
    for j in 0..MAX {
        set(rom, at(IDENT) + j, FIRST + j as u8);
        set(rom, at(LEVELS) + j, NO_LEVEL);
        set(rom, at(SHUTS) + j, 0);
    }
    set(rom, at(REP), 0);
    for (a, i) in SITES {
        for (k, b) in call(hook_at(i)).into_iter().enumerate() {
            set(rom, at(a) + k, b);
        }
    }
    for (k, b) in READ_POINT.into_iter().chain(call(hook_at(2))).enumerate() {
        set(rom, at(LEVEL_SITE) + k, b);
    }
    let pick = if rom[at(MAP_PICK)..at(MAP_PICK) + 3] == READ_POINT { vec![MAP_PICK] } else { vec![] };
    for a in REP_SITES.into_iter().chain(pick) {
        for (k, b) in read_point().into_iter().enumerate() {
            set(rom, at(a) + k, b);
        }
    }
    true
}

/// The added points' numbers.
pub fn numbers() -> impl Iterator<Item = u8> {
    (0..MAX as u8).map(|j| FIRST + j)
}

/// The number the map code sees for an added point of the world map: a world point with no
/// code of its own (0F:7024 moves Mario off $00).
pub const WORLD_REP: u8 = 0x01;
/// Zones (`zone_maps::ZONES`) that can have added points: all but the bonus game.
pub const ZONES: usize = 7;
/// $A68B (the map shown: 0 the world map) for each of those zones.
const SHOWN: [u8; ZONES] = [2, 1, 3, 4, 5, 6, 7];

/// The game zone shown, from $A68B (None: the world map or another map).
pub fn zone_shown(a68b: u8) -> Option<usize> {
    SHOWN.iter().position(|&v| v == a68b)
}

/// The number the map code sees for an added point of game zone `zone`.
pub fn zone_rep(zone: usize) -> u8 {
    zone_maps::ZONES[zone].point
}

/// Index of an added point (`FIRST` + index), if `p` is one.
pub fn index(p: u8) -> Option<usize> {
    (p >= FIRST && ((p - FIRST) as usize) < MAX).then(|| (p - FIRST) as usize)
}

/// The levels of the added points in place (`NO_LEVEL`: none).
pub fn levels(rom: &[u8]) -> Vec<u8> {
    if !installed(rom) {
        return vec![NO_LEVEL; MAX];
    }
    let o = offset(0x0F, LEVELS);
    rom[o..o + MAX].to_vec()
}

/// The pad directions each added point in place shuts until cleared.
pub fn shuts(rom: &[u8]) -> Vec<u8> {
    if !installed(rom) {
        return vec![0; MAX];
    }
    let o = offset(0x0F, SHUTS);
    rom[o..o + MAX].to_vec()
}

/// The number the map code sees for an added one: one of the map in place.
pub fn rep(rom: &[u8]) -> u8 {
    rom[offset(0x0F, REP)]
}

/// Write the levels and shut directions of the added points in place, and a number of the
/// map they are on (the hooks must be in).
pub fn write(rom: &mut [u8], levels: &[u8], shuts: &[u8], rep: u8) {
    for j in 0..MAX {
        set(rom, offset(0x0F, LEVELS) + j, levels.get(j).copied().unwrap_or(NO_LEVEL));
        set(rom, offset(0x0F, SHUTS) + j, shuts.get(j).copied().unwrap_or(0));
    }
    set(rom, offset(0x0F, REP), rep);
}

const STORE: usize = 0x24;
const MAGIC: &[u8; 4] = b"ADP1";

/// A game zone's added points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneAdded {
    /// Index into `zone_maps::ZONES` (below `ZONES`).
    pub zone: usize,
    /// Numbered from `FIRST`; their flag is their number.
    pub points: Vec<ZonePoint>,
}

/// The game zones' added points kept in this copy.
pub fn read_zones(rom: &[u8]) -> Vec<ZoneAdded> {
    let at = offset(STORE, 0x4000);
    if !is_expanded(rom) || &rom[at..at + 4] != MAGIC {
        return Vec::new();
    }
    let bank = &rom[at..at + BANK_SIZE];
    let mut i = 5;
    let mut v = Vec::new();
    for _ in 0..bank[4] {
        let (Some(&zone), Some(&n)) = (bank.get(i), bank.get(i + 1)) else { break };
        i += 2;
        let points: Option<Vec<ZonePoint>> = (0..n).map(|_| new_zones::decode_point(bank, &mut i)).collect();
        let Some(points) = points else { break };
        v.push(ZoneAdded { zone: zone as usize, points });
    }
    v
}

/// Keep the game zones' added points in the copy (zones without any are left out). Returns
/// the bytes used.
pub fn store_zones(rom: &mut [u8], zones: &[ZoneAdded]) -> Result<usize, CartError> {
    if !is_expanded(rom) {
        return Err(CartError::UnknownVersion);
    }
    let kept: Vec<&ZoneAdded> = zones.iter().filter(|z| !z.points.is_empty() && z.zone < ZONES).collect();
    let mut data = MAGIC.to_vec();
    data.push(kept.len() as u8);
    for z in &kept {
        if z.points.len() > MAX {
            return Err(CartError::TooManyPoints { max: MAX });
        }
        data.extend([z.zone as u8, z.points.len() as u8]);
        for p in &z.points {
            new_zones::encode_point(p, &mut data)?;
        }
    }
    if data.len() > BANK_SIZE {
        return Err(CartError::AddedPointsFull { need: data.len(), capacity: BANK_SIZE });
    }
    let at = offset(STORE, 0x4000);
    rom[at..at + BANK_SIZE].fill(0xFF);
    if !kept.is_empty() {
        rom[at..at + data.len()].copy_from_slice(&data);
    }
    Ok(if kept.is_empty() { 0 } else { data.len() })
}

/// Bytes to write over the copy (file offset, bytes) so that game zone `zone`'s added points
/// are in place instead of the world map's. `rom` is the copy as written.
pub fn writes_for_zone(rom: &[u8], zone: usize, points: &[ZonePoint]) -> Result<Vec<(usize, Vec<u8>)>, CartError> {
    if !installed(rom) || zone >= ZONES {
        return Err(CartError::UnknownVersion);
    }
    let mut n = rom.to_vec();
    let mut routes = map_routes::read(rom);
    let mut levels = world::point_levels(rom);
    let mut shuts = [0; MAX];
    for p in numbers() {
        routes.ways[p as usize] = Default::default();
        levels[p as usize] = NO_LEVEL;
    }
    for p in points {
        let Some(j) = index(p.number) else { return Err(CartError::PointOutsideZone { point: p.number }) };
        routes.ways[p.number as usize] = p.ways.clone();
        routes.places[p.number as usize] = p.place;
        levels[p.number as usize] = p.level;
        shuts[j] = p.shut;
    }
    map_routes::write(&mut n, &routes)?;
    write(&mut n, &levels[FIRST as usize..], &shuts, zone_rep(zone));
    Ok(new_zones::runs(rom, &n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_routes::{Place, Way, RIGHT};
    use crate::patch::EXPANDED_SIZE;

    #[test]
    fn the_hooks_fit_before_their_tables() {
        assert!(CODE + code().len() as u16 <= REP && WORLD_SHUT + WORLD_SHUT_CODE.len() as u16 <= 0x8000);
        assert_eq!(hook_at(HOOKS.len() - 1) + HOOK_LEN, CODE + code().len() as u16);
    }

    /// A 1 MB copy with the map code's sites as the game has them.
    fn copy() -> Vec<u8> {
        let mut r = vec![0u8; EXPANDED_SIZE];
        r[0x037C] = 0x01;
        r[0x39BD..0x39BD + 11].copy_from_slice(&[0x7C, 0xFE, 0xE0, 0xC2, 0x86, 0x03, 0xAF, 0xEA, 0x00, 0x40, 0xC9]);
        let at = |a: u16| offset(0x0F, a);
        for (a, i) in SITES {
            r[at(a)..at(a) + 3].copy_from_slice(&ld_hl(HOOKS[i].0));
        }
        r[at(LEVEL_SITE)..at(LEVEL_SITE) + 6].copy_from_slice(&[0x21, 0x18, 0x42, 0xFA, 0x40, 0xA8]);
        r[at(SHUT_TEST)..at(SHUT_TEST) + 4].copy_from_slice(&SHUT_TEST_BYTES);
        for a in REP_SITES.into_iter().chain([MAP_PICK]) {
            r[at(a)..at(a) + 3].copy_from_slice(&READ_POINT);
        }
        r
    }

    #[test]
    fn hooks_go_in_once_both_halves_with_nothing_added() {
        let mut rom = copy();
        assert!(!installed(&rom) && can_install(&rom));
        assert_eq!(levels(&rom), vec![NO_LEVEL; MAX], "no hooks: no added levels");
        assert!(install(&mut rom));
        assert!(installed(&rom));
        let at = offset(0x0F, 0x6EF6);
        assert_eq!(rom[at..at + 3], call(hook_at(0)));
        assert_eq!(rom[at + TWIN..at + TWIN + 3], call(hook_at(0)), "the twin half too");
        assert_eq!(rom[offset(0x0F, MAP_PICK)..offset(0x0F, MAP_PICK) + 3], read_point());
        assert_eq!((levels(&rom), shuts(&rom), rep(&rom)), (vec![NO_LEVEL; MAX], vec![0; MAX], 0));
        let once = rom.clone();
        assert!(install(&mut rom));
        assert!(rom == once);
        let mut other = copy();
        other[offset(0x0F, 0x41A9)] = 0x00;
        assert!(!can_install(&other) && !install(&mut other), "a map code it does not know");
    }

    #[test]
    fn the_shown_maps_levels_and_the_zones_points_are_kept() {
        let mut rom = copy();
        install(&mut rom);
        write(&mut rom, &[3, NO_LEVEL, 7], &[0, 0x10], zone_rep(1));
        assert_eq!((levels(&rom)[..4].to_vec(), shuts(&rom)[1], rep(&rom)), (vec![3, NO_LEVEL, 7, NO_LEVEL], 0x10, 0x28));
        assert_eq!((zone_shown(2), zone_shown(0), index(0x80), index(0x7F), index(FIRST + MAX as u8)), (Some(0), None, Some(0), None, None));
        let point = |n: u8| ZonePoint { number: n, place: Place { cam_x: 0, cam_y: 0, x: 40, y: 60, rest: [0, 0] }, ways: [Some(Way { steps: vec![RIGHT; 2], to: 0x20 }), None, None, None], level: 4, flag: n, shut: 0, pass: None };
        let zones = vec![ZoneAdded { zone: 0, points: vec![point(0x80), point(0x81)] }, ZoneAdded { zone: 3, points: vec![] }, ZoneAdded { zone: 5, points: vec![point(0x80)] }];
        assert!(read_zones(&rom).is_empty());
        store_zones(&mut rom, &zones).unwrap();
        assert_eq!(read_zones(&rom), vec![zones[0].clone(), zones[2].clone()], "zones without added points are left out");
        let too_many = ZoneAdded { zone: 1, points: vec![point(0x80); MAX + 1] };
        assert_eq!(store_zones(&mut rom, &[too_many]), Err(CartError::TooManyPoints { max: MAX }));
        store_zones(&mut rom, &[]).unwrap();
        assert!(read_zones(&rom).is_empty());
    }
}
