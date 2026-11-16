//! Four bank-0 tables of 32 bytes the game reads by level number ($A269) when a level loads
//! (research `physicsprobe`, `tableprobe`, `cameraprobe`, `scrollprobe`, `animationprobe`):
//! - 0:1F91 → $A287: gravity; any of bits 0 and 3 gives the Space Zone's floating jumps
//!   (levels 18 and 28 have 8, level 19 has 1). Bits 1 and 2 are the game's own while playing.
//! - 0:1F71 → $A2C8: auto-scroll (no level has it): the camera moves right 0.5 px a frame
//!   and pushes Mario, who loses a life caught against a wall (0:0B52, 0:1202, 0:1464).
//! - 0:1F51 → $A2D9: camera margin: with it the camera moves up when Mario is about 23 px
//!   lower on the screen (0:0D4F, 0:0D74).
//! - 0:35EA → $A299 = value + 1: animated tiles, every 2 x high nibble frames four tiles from
//!   bank $1D $6680 + low nibble x $100 to $9200 (0:3873); $FF none.
//!
//! An expanded copy keeps them in bank 0 and its twin (bank $20), as code in bank 0 can run
//! with the upper bank bit set.
use crate::patch::TWIN;
use crate::rom::offset;

const GRAVITY: u16 = 0x1F91;
const SCROLL: u16 = 0x1F71;
const CAMERA: u16 = 0x1F51;
const ANIMATION: u16 = 0x35EA;
/// The levels the tables have.
pub const LEVELS: usize = 32;

pub const SPACE_GRAVITY: u8 = 0x08;
pub const SCROLL_ON: u8 = 0x01;
pub const WIDE_CAMERA: u8 = 0x08;
pub const NO_ANIMATION: u8 = 0xFF;

/// A level's entries in the four tables, as the game keeps them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tables {
    pub gravity: u8,
    pub scroll: u8,
    pub camera: u8,
    pub animation: u8,
}

impl Default for Tables {
    fn default() -> Self {
        Tables { gravity: 0, scroll: 0, camera: 0, animation: NO_ANIMATION }
    }
}

const ORDER: [u16; 4] = [GRAVITY, SCROLL, CAMERA, ANIMATION];

impl Tables {
    /// Level `level`'s entries (0-31).
    pub fn read(rom: &[u8], level: usize) -> Tables {
        let b = ORDER.map(|t| rom[offset(0, t) + level]);
        Tables::from_bytes(b)
    }

    /// The writes (file offset, byte) that give level `level` these entries, in bank 0 and,
    /// in an expanded copy, its twin.
    pub fn writes(&self, rom: &[u8], level: usize) -> Vec<(usize, Vec<u8>)> {
        let mut w = Vec::new();
        for (t, v) in ORDER.iter().zip(self.bytes()) {
            let at = offset(0, *t) + level;
            w.push((at, vec![v]));
            if rom.len() > TWIN {
                w.push((at + TWIN, vec![v]));
            }
        }
        w
    }

    pub fn write(&self, rom: &mut [u8], level: usize) {
        for (at, b) in self.writes(rom, level) {
            rom[at] = b[0];
        }
    }

    pub fn bytes(&self) -> [u8; 4] {
        [self.gravity, self.scroll, self.camera, self.animation]
    }

    pub fn from_bytes(b: [u8; 4]) -> Tables {
        Tables { gravity: b[0], scroll: b[1], camera: b[2], animation: b[3] }
    }

    pub fn space_gravity(&self) -> bool {
        self.gravity & 0x09 != 0
    }

    /// Space or normal gravity; the game's own space value is kept when it has one.
    pub fn set_space_gravity(&mut self, on: bool, game: &Tables) {
        self.gravity = match (on, game.space_gravity()) {
            (true, true) => game.gravity,
            (true, false) => SPACE_GRAVITY,
            (false, _) => self.gravity & !0x09,
        };
    }

    pub fn scrolls(&self) -> bool {
        self.scroll != 0
    }

    pub fn set_scrolls(&mut self, on: bool) {
        self.scroll = if on { SCROLL_ON } else { 0 };
    }

    pub fn wide_camera(&self) -> bool {
        self.camera != 0
    }

    pub fn set_wide_camera(&mut self, on: bool) {
        self.camera = if on { WIDE_CAMERA } else { 0 };
    }

    pub fn animated(&self) -> bool {
        self.animation != NO_ANIMATION
    }

    /// The level's own animation (`game`'s) or none.
    pub fn set_animated(&mut self, on: bool, game: &Tables) {
        self.animation = if on { game.animation } else { NO_ANIMATION };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::EXPANDED_SIZE;

    #[test]
    fn written_into_bank_0_and_its_twin_and_read_back() {
        let mut rom = vec![0u8; EXPANDED_SIZE];
        let t = Tables { gravity: 8, scroll: 1, camera: 8, animation: 0x4D };
        t.write(&mut rom, 3);
        assert_eq!(Tables::read(&rom, 3), t);
        assert_eq!(rom[TWIN + 0x1F91 + 3], 8);
        assert_eq!(Tables::read(&rom, 2), Tables::from_bytes([0; 4]));
    }

    #[test]
    fn choices_keep_the_games_own_values() {
        let game = Tables { gravity: 1, scroll: 0, camera: 0, animation: 0x4D };
        let mut t = game;
        t.set_space_gravity(false, &game);
        assert!(!t.space_gravity());
        t.set_space_gravity(true, &game);
        assert_eq!(t.gravity, 1, "level 19's own value");
        let plain = Tables::default();
        let mut u = plain;
        u.set_space_gravity(true, &plain);
        assert_eq!(u.gravity, SPACE_GRAVITY);
        t.set_animated(false, &game);
        assert!(!t.animated());
        t.set_animated(true, &game);
        assert_eq!(t.animation, 0x4D);
    }
}
