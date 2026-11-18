//! Where every new game begins: a save slot keeps 64 bytes of map state, the first Mario's map
//! point, and a new slot clears them (0:2914 `ld b,$40 / xor a / ld (hl+),a / dec b / jr nz`),
//! so the game starts on point $00 of the world map; loading a game shows the map its point is
//! on, the world map or a zone (research `startpoint`, `continueat`, `newstart`). With another
//! start, 0:2914 becomes `call $1967` (and four `nop`s), and 0:1967, a run the game never runs
//! or reads (research `unusedcheck`), writes the point first, then the 63 zeros. The byte after
//! that code names the new zone the point is in (index + 1; 0: none), for this port's game,
//! which shows the new zone in its base's place (`new_zones`).
use crate::patch::TWIN;

/// Mario's start: a map point, and the new zone it is in (None: the world map or a game zone,
/// as the point's number says).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Start {
    pub point: u8,
    pub new_zone: Option<usize>,
}

const SITE: usize = 0x2914;
const CLEAR: [u8; 7] = [0x06, 0x40, 0xAF, 0x22, 0x05, 0x20, 0xFC];
const STUB_AT: usize = 0x1967;
const CALL: [u8; 7] = [0xCD, STUB_AT as u8, (STUB_AT >> 8) as u8, 0x00, 0x00, 0x00, 0x00];
const STUB_LEN: usize = 11;
const ZONE_AT: usize = STUB_AT + STUB_LEN;

fn stub(point: u8) -> [u8; STUB_LEN] {
    [0x3E, point, 0x22, 0x06, 0x3F, 0xAF, 0x22, 0x05, 0x20, 0xFC, 0xC9]
}

/// The start in `rom` (the game's own when it has no patch).
pub fn read(rom: &[u8]) -> Start {
    if rom.get(SITE..SITE + CALL.len()) != Some(&CALL[..]) {
        return Start::default();
    }
    Start { point: rom[STUB_AT + 1], new_zone: rom[ZONE_AT].checked_sub(1).map(usize::from) }
}

/// Can `rom` take the patch (the game's code at the site, the room free or ours)?
pub fn fits(rom: &[u8]) -> bool {
    let site = &rom[SITE..SITE + CLEAR.len()];
    let room = &rom[STUB_AT..ZONE_AT + 1];
    (site == CLEAR || site == CALL) && (room.iter().all(|&b| b == 0xFF) || (room[0] == stub(0)[0] && room[2..STUB_LEN] == stub(0)[2..]))
}

/// Make every new game begin at `start` (the game's own start puts the game's bytes back),
/// in bank 0 and, in an expanded copy, its twin. False when the cartridge cannot take it.
pub fn write(rom: &mut [u8], start: Start) -> bool {
    if !fits(rom) {
        return false;
    }
    let halves: &[usize] = if rom.len() > TWIN { &[0, TWIN] } else { &[0] };
    for &base in halves {
        if start == Start::default() {
            rom[base + SITE..base + SITE + CLEAR.len()].copy_from_slice(&CLEAR);
            rom[base + STUB_AT..base + ZONE_AT + 1].fill(0xFF);
        } else {
            rom[base + SITE..base + SITE + CALL.len()].copy_from_slice(&CALL);
            rom[base + STUB_AT..base + ZONE_AT].copy_from_slice(&stub(start.point));
            rom[base + ZONE_AT] = start.new_zone.map_or(0, |k| k as u8 + 1);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom() -> Vec<u8> {
        let mut r = vec![0u8; 0x8000];
        r[SITE..SITE + CLEAR.len()].copy_from_slice(&CLEAR);
        r[STUB_AT..STUB_AT + 44].fill(0xFF);
        r
    }

    #[test]
    fn a_start_is_written_read_and_taken_back() {
        let mut r = rom();
        assert_eq!(read(&r), Start::default());
        let s = Start { point: 0x29, new_zone: Some(2) };
        assert!(write(&mut r, s));
        assert_eq!(read(&r), s);
        assert_eq!(r[SITE..SITE + 3], [0xCD, 0x67, 0x19]);
        assert!(write(&mut r, Start::default()));
        assert_eq!(r, rom(), "the game's own start: the game's bytes back");
    }

    #[test]
    fn another_cartridge_is_left_alone() {
        let mut r = vec![0u8; 0x8000];
        assert!(!write(&mut r, Start { point: 3, new_zone: None }));
        assert_eq!(r, vec![0u8; 0x8000]);
    }
}
