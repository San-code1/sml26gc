//! Where a level's secret exit (block $4B, `level_goals`) takes Mario.
//!
//! After a level the map code (0F:6F56) sees $A2B4 set by the secret exit and, unless $A224
//! is set, changes $A840 from the level's point to the point the game's secret way starts
//! from ($48 > $47, $05/$12/$1A > $69, $29 > $2C, ...); 0F:6FB2 then calls 0F:6EF0, which does
//! not mark that point cleared, 0F:7003 shows the map of the new number, and the zone's
//! handler (0F:518E, 52A3, 540F, 5533, 569C) puts Mario on its fixed point and opens the
//! secret path.
//!
//! An edited copy can send a level's secret exit elsewhere: 0F:6F62 `ld a,($A840)` becomes a
//! call to `HOOK`, which looks the level just played ($A269) up in a table (level, target
//! point, new zone or $FF); for a level in it it puts the target in $A840, calls 0F:6EF0 as
//! the game does, clears $A2B4 so no zone handler moves Mario again, sets $A224 as the game's
//! own handlers do when they place Mario (so 0F:432D does not walk him on along the way his
//! point opens, as it does after a level), and goes on at 0F:6FB5 (Mario on the target, its
//! map shown). Other levels keep the game's way. A new zone's
//! target is its door (the port enters the zone from there). A new level is played as its
//! base level, so while it is in place the base's entry is the new level's (`stand_in`).
use crate::error::CartError;
use crate::patch::TWIN;
use crate::rom::offset;

/// 0F:6F62 `ld a,($A840)`, in the secret exit's way after a level.
const SITE: u16 = 0x6F62;
const SITE_BYTES: [u8; 3] = [0xFA, 0x40, 0xA8];
/// Unused space in bank $0F after the new zones' hook.
const HOOK: u16 = 0x7E40;
const TABLE: u16 = HOOK + 42;
/// The table's last byte (`added_points` follows).
const LAST: u16 = 0x7EFF;
const LIST_END: u8 = 0xFF;
const ENTRY: usize = 3;
const NO_ZONE: u8 = 0xFF;

/// Where a secret exit leads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecretTo {
    /// A point of the world map or of a game zone.
    Point(u8),
    /// The editor's new zone k, entered on its entry.
    NewZone(usize),
}

/// The game's own secret exits: level, the point Mario ends on (`research secretprobe`).
pub const GAME: [(usize, u8); 7] = [(2, 0x25), (7, 0x2D), (8, 0x7A), (15, 0x65), (17, 0x07), (18, 0x44), (20, 0x6A)];

fn set(rom: &mut [u8], o: usize, v: u8) {
    rom[o] = v;
    if rom.len() >= 2 * TWIN && o < TWIN {
        rom[o + TWIN] = v;
    }
}

/// The call and this hook are in place (a copy with another hook there keeps no targets).
fn hooked(rom: &[u8]) -> bool {
    let (at, code) = (offset(0x0F, SITE), offset(0x0F, HOOK));
    rom.len() > offset(0x0F, LAST) && rom[at..at + 3] == [0xCD, HOOK as u8, (HOOK >> 8) as u8] && rom[code..code + hook_code().len()] == hook_code()[..]
}

/// The table's entries (level, target point, new zone or `NO_ZONE`).
fn entries(rom: &[u8]) -> Vec<[u8; ENTRY]> {
    if !hooked(rom) {
        return Vec::new();
    }
    let (mut at, end) = (offset(0x0F, TABLE), offset(0x0F, LAST));
    let mut v = Vec::new();
    while at + ENTRY <= end && rom[at] != LIST_END {
        v.push([rom[at], rom[at + 1], rom[at + 2]]);
        at += ENTRY;
    }
    v
}

/// The levels' secret exit targets the copy keeps (none: the game's own way for all).
pub fn read(rom: &[u8]) -> Vec<(usize, SecretTo)> {
    entries(rom).into_iter().map(|[l, p, k]| (l as usize, if k == NO_ZONE { SecretTo::Point(p) } else { SecretTo::NewZone(k as usize) })).collect()
}

/// Write the levels' targets into the copy (`doors`: each new zone's door point). No
/// targets: the game's code as it was.
pub fn write(rom: &mut [u8], targets: &[(usize, SecretTo)], doors: &[u8]) -> Result<(), CartError> {
    let at = offset(0x0F, SITE);
    let ours = rom.len() > offset(0x0F, LAST) && rom[at..at + 3] == [0xCD, HOOK as u8, (HOOK >> 8) as u8];
    if targets.is_empty() {
        if ours {
            for (i, b) in SITE_BYTES.into_iter().enumerate() {
                set(rom, at + i, b);
            }
        }
        return Ok(());
    }
    if rom.len() <= offset(0x0F, LAST) || !(rom[at..at + 3] == SITE_BYTES || ours) {
        return Err(CartError::UnknownVersion);
    }
    let mut data = Vec::new();
    for &(l, to) in targets {
        let entry = match to {
            SecretTo::Point(p) => [l as u8, p, NO_ZONE],
            SecretTo::NewZone(k) => match doors.get(k) {
                Some(&d) => [l as u8, d, k as u8],
                None => continue,
            },
        };
        data.extend(entry);
    }
    data.push(LIST_END);
    let capacity = (LAST + 1 - TABLE) as usize;
    if data.len() > capacity {
        return Err(CartError::SecretExitsFull { need: data.len(), capacity });
    }
    for (i, &c) in hook_code().iter().chain(&data).enumerate() {
        set(rom, offset(0x0F, HOOK) + i, c);
    }
    for (i, c) in [0xCD, HOOK as u8, (HOOK >> 8) as u8].into_iter().enumerate() {
        set(rom, at + i, c);
    }
    Ok(())
}

/// Writes that give base level `base`'s entry the target of new level `level` (or none),
/// while the new level is played in its place.
pub fn stand_in(rom: &[u8], level: usize, base: usize) -> Vec<(usize, Vec<u8>)> {
    if !hooked(rom) {
        return Vec::new();
    }
    let all = entries(rom);
    let mut v: Vec<[u8; ENTRY]> = all.iter().filter(|e| e[0] as usize != base).copied().collect();
    if let Some(e) = all.iter().find(|e| e[0] as usize == level) {
        v.insert(0, [base as u8, e[1], e[2]]);
    }
    let mut data: Vec<u8> = v.into_iter().flatten().collect();
    data.resize(all.len() * ENTRY + 1, LIST_END);
    let at = offset(0x0F, TABLE);
    let mut w = vec![(at, data.clone())];
    if rom.len() >= 2 * TWIN {
        w.push((at + TWIN, data));
    }
    w
}

/// ld a,($A269) / ld hl,TABLE / next: ld b,(hl) / inc b / jr z,keep / dec b / inc hl /
/// cp b / jr z,found / inc hl / inc hl / jr next / found: ld a,(hl) / ld ($A840),a / pop hl /
/// call $6EF0 / xor a / ld ($A2B4),a / dec a / ld ($A224),a / jp $6FB5 / keep: ld a,($A840) /
/// ret (as the game had it).
fn hook_code() -> Vec<u8> {
    let t = TABLE.to_le_bytes();
    vec![
        0xFA, 0x69, 0xA2, 0x21, t[0], t[1], 0x46, 0x04, 0x28, 0x1C, 0x05, 0x23, 0xB8, 0x28, 0x04, 0x23, 0x23, 0x18, 0xF3, 0x7E, 0xEA, 0x40, 0xA8, 0xE1, 0xCD, 0xF0, 0x6E, 0xAF, 0xEA, 0xB4, 0xA2, 0x3D, 0xEA, 0x24, 0xA2, 0xC3, 0xB5, 0x6F, 0xFA, 0x40, 0xA8, 0xC9,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x4_0000];
        rom[offset(0x0F, SITE)..offset(0x0F, SITE) + 3].copy_from_slice(&SITE_BYTES);
        rom
    }

    #[test]
    fn the_hook_is_as_long_as_its_room() {
        assert_eq!(hook_code().len(), (TABLE - HOOK) as usize);
    }

    #[test]
    fn targets_are_written_read_back_and_taken_away() {
        let mut rom = rom();
        let targets = [(7, SecretTo::Point(0x07)), (33, SecretTo::NewZone(1))];
        write(&mut rom, &targets, &[0x6D, 0x7B]).unwrap();
        assert_eq!(read(&rom), targets);
        let t = offset(0x0F, TABLE);
        assert_eq!(rom[t..t + 7], [7, 0x07, NO_ZONE, 33, 0x7B, 1, LIST_END], "a new zone's target is its door");
        write(&mut rom, &[], &[]).unwrap();
        assert_eq!(rom[offset(0x0F, SITE)..offset(0x0F, SITE) + 3], SITE_BYTES);
        assert!(read(&rom).is_empty());
    }

    #[test]
    fn a_new_level_takes_its_base_entry_while_in_place() {
        let mut rom = rom();
        write(&mut rom, &[(0, SecretTo::Point(0x07)), (33, SecretTo::Point(0x28))], &[]).unwrap();
        let t = offset(0x0F, TABLE);
        assert_eq!(stand_in(&rom, 33, 0), vec![(t, vec![0, 0x28, NO_ZONE, 33, 0x28, NO_ZONE, LIST_END])]);
        assert_eq!(stand_in(&rom, 34, 0), vec![(t, vec![33, 0x28, NO_ZONE, LIST_END, LIST_END, LIST_END, LIST_END])], "a new level without one: its base has none either");
    }
}
