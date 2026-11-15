//! Levels beyond the cartridge's 32 (numbers 32 and up), kept in the 1 MB edited copy.
//!
//! The game has room for 32 levels only: its level number ($A269) indexes about ten tables
//! of 32 entries (header, objects, music, graphics, time...; see `legacy/gb_port/MECHANICS.md`). A new level
//! therefore is played as a stand-in: it is based on one of the 32 (`base`), whose number
//! the game sees and whose music, graphics and other table values it uses, and it brings
//! its own header, map and objects. Entering it, the port writes those over the base
//! level's in the running console's ROM (`writes_for`, `apply`) and puts the base level's
//! back once Mario is on the map again; the game loads the level as usual and every reload
//! (a lost life) gets the new one too. So these levels play only in this port, from the
//! port's own world map.
//!
//! What is written while one is played:
//! * the base level's 20-byte header (bank 1, table $55CB);
//! * its map: the compressed data at $4310 of the twin of the header's bank (just after
//!   the map table at $42F0, which the decoder alone reads), and that table entry in both
//!   the bank and its twin;
//! * its objects: the list at $4000 of bank $23 and the base's entry of the table at $6037.
//!
//! Storage: twins of banks nothing reads from ($38-$3F, `STORE`). The first holds a
//! directory: "XLV1", the count, then per level the base, the header, where its map is
//! (bank, address, length) and where its objects are (bank, address); then "XLN1" and each
//! level's name (`NAME_MAX` bytes, zero-padded; a copy without them has no names); the data
//! follows.
use crate::error::CartError;
use crate::levels::{self, decode, encode, MAP_TABLE};
use crate::objects::{self, Obj};
use crate::patch::{is_expanded, objects_patched, TWIN};
use crate::rom::{offset, set_word, word, BANK_SIZE};

/// Number of the first new level.
pub const FIRST: usize = 32;
/// How many new levels there can be.
pub const MAX: usize = 64;
/// Banks that store the new levels (twins of banks the game reads only with the upper
/// bank bit clear).
const STORE: [usize; 8] = [0x3F, 0x3E, 0x3D, 0x3C, 0x3B, 0x3A, 0x39, 0x38];
const MAGIC: &[u8; 4] = b"XLV1";
const ENTRY: usize = 29;
const NAMES_MAGIC: &[u8; 4] = b"XLN1";
const NAMES: usize = 5 + MAX * ENTRY;
const DIRECTORY: usize = NAMES + 4 + MAX * NAME_MAX;
/// The longest name a new level or zone can have (printable ASCII).
pub const NAME_MAX: usize = 24;
/// Where a new level's map goes while it is played, and the room it has there.
const MAP_AT: u16 = 0x4310;
pub const MAP_ROOM: usize = 0x8000 - MAP_AT as usize;
/// Room for a new level's objects while it is played (bank $23 $4000-$6036).
pub const OBJECTS_ROOM: usize = 0x6037 - 0x4000;

#[derive(Clone, Debug, PartialEq)]
pub struct Extra {
    /// The level of the 32 it stands in for.
    pub base: u8,
    pub header: [u8; 20],
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
    /// Given in the editor; empty: none.
    pub name: String,
}

/// `name` as the copy can keep it: printable ASCII, at most `NAME_MAX` characters.
pub fn clean_name(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).take(NAME_MAX).collect()
}

pub(crate) fn name_from(bytes: &[u8]) -> String {
    clean_name(&String::from_utf8_lossy(bytes.split(|&b| b == 0).next().unwrap_or_default()))
}

/// A new level that starts as a copy of level `base`.
pub fn based_on(rom: &[u8], base: u8) -> Extra {
    let h = levels::header(rom, base as usize);
    let (_, start) = levels::map_location(rom, h);
    let mut header = [0u8; 20];
    header.copy_from_slice(&rom[h.at..h.at + 20]);
    Extra { base, header, map: decode(rom, start).0, objs: objects::read(rom, base as usize).0, name: String::new() }
}

fn dir() -> usize {
    offset(STORE[0], 0x4000)
}

/// The new levels kept in this copy (none in a copy that has no room for them).
pub fn read(rom: &[u8]) -> Vec<Extra> {
    if !is_expanded(rom) || &rom[dir()..dir() + 4] != MAGIC {
        return Vec::new();
    }
    let d = dir();
    let n = (rom[d + 4] as usize).min(MAX);
    let named = &rom[d + NAMES..d + NAMES + 4] == NAMES_MAGIC;
    (0..n)
        .map(|k| {
            let e = d + 5 + k * ENTRY;
            let mut header = [0u8; 20];
            header.copy_from_slice(&rom[e + 1..e + 21]);
            let map_at = offset(rom[e + 21] as usize, word(rom, e + 22));
            let objs_at = offset(rom[e + 26] as usize, word(rom, e + 27));
            let mut objs = Vec::new();
            let mut i = objs_at;
            while rom[i] != 0xFF && objs.len() < 400 {
                objs.push(Obj::from_bytes(rom[i], rom[i + 1], rom[i + 2]));
                i += 3;
            }
            let name = if named { name_from(&rom[d + NAMES + 4 + k * NAME_MAX..][..NAME_MAX]) } else { String::new() };
            Extra { base: rom[e], header, map: decode(rom, map_at).0, objs, name }
        })
        .collect()
}

fn object_bytes(objs: &[Obj]) -> Vec<u8> {
    let mut v: Vec<Obj> = objs.to_vec();
    v.sort_by_key(|o| o.x); // the order the game keeps them in
    let mut b: Vec<u8> = v.iter().flat_map(|o| o.to_bytes()).collect();
    b.push(0xFF);
    b
}

/// Write the new levels into the copy (replacing the ones it had). Returns the bytes used.
pub fn store(rom: &mut [u8], extras: &[Extra]) -> Result<usize, CartError> {
    if !is_expanded(rom) || !objects_patched(rom) {
        return Err(CartError::UnknownVersion);
    }
    if extras.len() > MAX {
        return Err(CartError::TooManyLevels { max: FIRST + MAX });
    }
    let mut datas = Vec::new();
    for (k, e) in extras.iter().enumerate() {
        let (map, objs) = (encode(&e.map), object_bytes(&e.objs));
        if map.len() > MAP_ROOM || objs.len() > OBJECTS_ROOM {
            return Err(CartError::LevelTooBig { level: FIRST + k, need: map.len().max(objs.len()), capacity: if map.len() > MAP_ROOM { MAP_ROOM } else { OBJECTS_ROOM } });
        }
        datas.push((map, objs));
    }
    // place everything one after another, never across the end of a bank
    let (mut bank, mut at) = (0usize, DIRECTORY);
    let mut place = |len: usize| -> Option<(u8, u16)> {
        if at + len > BANK_SIZE {
            bank += 1;
            at = 0;
        }
        let b = *STORE.get(bank)?;
        let r = (b as u8, 0x4000 + at as u16);
        at += len;
        Some(r)
    };
    let mut writes = Vec::new();
    let mut used = DIRECTORY;
    for (map, objs) in &datas {
        let m = place(map.len());
        let o = place(objs.len());
        let (Some(m), Some(o)) = (m, o) else {
            let capacity = STORE.len() * BANK_SIZE;
            return Err(CartError::LevelsFull { need: used + map.len() + objs.len(), capacity });
        };
        used += map.len() + objs.len();
        writes.push((m, o));
    }
    for b in STORE {
        rom[offset(b, 0x4000)..offset(b, 0x4000) + BANK_SIZE].fill(0xFF);
    }
    let d = dir();
    rom[d..d + 4].copy_from_slice(MAGIC);
    rom[d + 4] = extras.len() as u8;
    rom[d + NAMES..d + NAMES + 4].copy_from_slice(NAMES_MAGIC);
    rom[d + NAMES + 4..d + DIRECTORY].fill(0);
    for (k, e) in extras.iter().enumerate() {
        let name = clean_name(&e.name);
        rom[d + NAMES + 4 + k * NAME_MAX..][..name.len()].copy_from_slice(name.as_bytes());
    }
    for (k, (e, ((m, o), (map, objs)))) in extras.iter().zip(writes.iter().zip(&datas)).enumerate() {
        let en = d + 5 + k * ENTRY;
        rom[en] = e.base;
        rom[en + 1..en + 21].copy_from_slice(&e.header);
        rom[en + 21] = m.0;
        set_word(rom, en + 22, m.1);
        set_word(rom, en + 24, map.len() as u16);
        rom[en + 26] = o.0;
        set_word(rom, en + 27, o.1);
        let ma = offset(m.0 as usize, m.1);
        rom[ma..ma + map.len()].copy_from_slice(map);
        let oa = offset(o.0 as usize, o.1);
        rom[oa..oa + objs.len()].copy_from_slice(objs);
    }
    Ok(used)
}

/// Bytes to write over the base level's (file offset, bytes) so that the game loads `e`
/// when it loads its base level.
pub fn writes_for(rom: &[u8], e: &Extra) -> Vec<(usize, Vec<u8>)> {
    let h = levels::header(rom, e.base as usize);
    let bank = e.header[0x0D] as usize;
    let map = (e.header[0x12] & 0x0F) as usize;
    let entry = offset(bank, MAP_TABLE) + map * 2;
    let to = [MAP_AT as u8, (MAP_AT >> 8) as u8].to_vec();
    let obj_bank = objects::bank(rom);
    vec![
        (h.at, e.header.to_vec()),
        (offset(bank | 0x20, MAP_AT), encode(&e.map)),
        (entry, to.clone()),
        (entry + TWIN, to),
        (offset(obj_bank, 0x4000), object_bytes(&e.objs)),
        (offset(obj_bank, 0x6037) + e.base as usize * 2, vec![0x00, 0x40]),
    ]
}

/// Make the writes; returns the writes that undo them.
pub fn apply(rom: &mut [u8], writes: &[(usize, Vec<u8>)]) -> Vec<(usize, Vec<u8>)> {
    let mut undo = Vec::new();
    for (at, bytes) in writes {
        undo.push((*at, rom[*at..*at + bytes.len()].to_vec()));
        rom[*at..*at + bytes.len()].copy_from_slice(bytes);
    }
    undo.reverse();
    undo
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::EXPANDED_SIZE;

    /// An expanded-looking copy with a header for level 2 and its map.
    fn fake() -> Vec<u8> {
        let mut r = vec![0u8; EXPANDED_SIZE];
        r[0x037C] = 0x01;
        r[0x39BD..0x39BD + 11].copy_from_slice(&[0x7C, 0xFE, 0xE0, 0xC2, 0x86, 0x03, 0xAF, 0xEA, 0x00, 0x40, 0xC9]);
        r[0x0581..0x0584].copy_from_slice(&[0xCD, 0xC8, 0x39]);
        r[0x39C8..0x39C8 + 13].copy_from_slice(&[0x3E, 0x01, 0xEA, 0x00, 0x40, 0xCD, 0xBB, 0x6B, 0xAF, 0xEA, 0x00, 0x40, 0xC9]);
        set_word(&mut r, 0x55CB + 4, 0x6000);
        r[0x6000 + 0x0D] = 0x09;
        r[0x6000 + 0x12] = 0x01;
        set_word(&mut r, offset(0x09, MAP_TABLE) + 2, 0x5000);
        let empty = encode(&vec![0u8; levels::MAP_BYTES]);
        let at = offset(0x29, 0x5000);
        r[at..at + empty.len()].copy_from_slice(&empty);
        set_word(&mut r, offset(0x23, 0x6037) + 4, 0x4100);
        r[offset(0x23, 0x4100)] = 0xFF;
        r
    }

    #[test]
    fn stored_levels_read_back_and_play_over_their_base() {
        let mut rom = fake();
        assert!(read(&rom).is_empty());
        let mut e = based_on(&rom, 2);
        assert!(e.objs.is_empty() && e.map.iter().all(|&b| b == 0));
        e.map[300] = 7;
        e.objs.push(Obj { x: 64, y: 40, kind: 5, flag: false });
        let other = based_on(&rom, 2);
        let used = store(&mut rom, &[e.clone(), other.clone()]).unwrap();
        assert!(used > DIRECTORY);
        assert_eq!(read(&rom), vec![e.clone(), other]);

        // played: the game finds the new map and objects where it looks for level 2's
        let before = rom.clone();
        let w = writes_for(&rom, &e);
        let undo = apply(&mut rom, &w);
        let h = levels::header(&rom, 2);
        let (_, start) = levels::map_location(&rom, h);
        assert_eq!(decode(&rom, start).0, e.map);
        assert_eq!(objects::read(&rom, 2).0, e.objs);
        apply(&mut rom, &undo);
        assert!(rom == before, "the base level is back");
    }

    #[test]
    fn too_many_levels_are_refused() {
        let mut rom = fake();
        let e = based_on(&rom, 2);
        assert_eq!(store(&mut rom, &vec![e; MAX + 1]), Err(CartError::TooManyLevels { max: FIRST + MAX }));
    }
}
