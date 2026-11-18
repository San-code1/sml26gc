//! Where Mario begins a level: header bytes $00-$07 (see `levels`), four little-endian words
//! in level pixels. 00:0F2A puts Mario's y and x in $FFC0 and $FFC2, 00:051B the camera's y
//! and x in $FFC8 and $FFCA. The camera is the middle of the screen (SCY = camera y - $48,
//! SCX = camera x - $50). Mario's sprite is 16x24 with its left edge at his x and its top at
//! his y + 6; his feet are at y + 28 (every start of the game has y = 4 mod 16).
//! When the midway bell was rung (bit 3 of $A27D) 01:55AB copies 01:377B + 8*level over
//! these 8 bytes instead.
use crate::levels::{COLS, ROWS};
use crate::rom::{set_word, word};

pub const SPRITE_W: u16 = 16;
pub const SPRITE_H: u16 = 24;
/// From Mario's y to the top of his sprite, and to his feet.
pub const SPRITE_TOP: u16 = 6;
pub const FEET: u16 = 28;
/// The camera's limits: the screen inside the level.
const CAM_X_MIN: u16 = 0x50;
const CAM_X_MAX: u16 = (COLS * 16) as u16 - 0x50;
const CAM_Y_MIN: u16 = 0x48;
const CAM_Y_MAX: u16 = 0x2C0;
/// Where the game's own headers put the camera from Mario.
const CAM_AHEAD: u16 = 0x21;
const CAM_ABOVE: u16 = 0x14;
/// How far past the screen's left and right edges the objects a level begins with are already
/// passed (research `startobjects`).
const PASSED_LEFT: u16 = 33;
const PASSED_RIGHT: u16 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Start {
    pub mario_x: u16,
    pub mario_y: u16,
    pub camera_x: u16,
    pub camera_y: u16,
}

impl Start {
    /// Mario's start from a header (its first 8 bytes at least).
    pub fn read(header: &[u8]) -> Start {
        Start { mario_y: word(header, 0), mario_x: word(header, 2), camera_y: word(header, 4), camera_x: word(header, 6) }
    }

    pub fn write(&self, header: &mut [u8]) {
        set_word(header, 0, self.mario_y);
        set_word(header, 2, self.mario_x);
        set_word(header, 4, self.camera_y);
        set_word(header, 6, self.camera_x);
    }

    /// Mario with his left edge at `x` and his feet on the top of the block row under level
    /// pixel `feet_y`, the camera where the game's own levels put it.
    pub fn standing(x: u16, feet_y: u16) -> Start {
        let max_x = (COLS * 16) as u16 - SPRITE_W;
        let mario_x = x.min(max_x);
        let feet = (feet_y / 16 * 16).clamp(FEET + 4, (ROWS * 16) as u16);
        let mario_y = feet - FEET;
        Start {
            mario_x,
            mario_y,
            camera_x: (mario_x + CAM_AHEAD).clamp(CAM_X_MIN, CAM_X_MAX),
            camera_y: mario_y.saturating_sub(CAM_ABOVE).clamp(CAM_Y_MIN, CAM_Y_MAX),
        }
    }

    /// The band of the level, from x `.0` to x `.1` (both in) and all its height, whose objects
    /// do not appear when the level begins. The game brings an object to life when its x comes
    /// in a little past the screen's left or right edge, whatever its height, and these are
    /// already past: they appear only once Mario has gone far enough away and comes back
    /// (research `startobjects`; none of the game's levels has one there, `firstscreen`).
    pub fn passed_band(&self) -> (u16, u16) {
        (self.camera_x.saturating_sub(0x50 + PASSED_LEFT), self.camera_x + 0x50 + PASSED_RIGHT)
    }

    /// Is an object at level x `x` missing when the level begins?
    pub fn passed_at_start(&self, x: u16) -> bool {
        let (l, r) = self.passed_band();
        (l..=r).contains(&x)
    }

    /// The top left corner of Mario's sprite, in level pixels.
    pub fn sprite_at(&self) -> (u16, u16) {
        (self.mario_x, self.mario_y + SPRITE_TOP)
    }

    /// Mario's body here is inside solid blocks or outside the level.
    pub fn blocked(&self, map: &[u8]) -> bool {
        let (x, top) = self.sprite_at();
        let bottom = self.mario_y + FEET - 1;
        if x as usize + SPRITE_W as usize > COLS * 16 || bottom as usize >= ROWS * 16 {
            return true;
        }
        let cols = [x / 16, (x + SPRITE_W - 1) / 16];
        (top / 16..=bottom / 16).any(|r| cols.iter().any(|&c| solid(map.get(r as usize * COLS + c as usize).copied().unwrap_or(0))))
    }
}

/// Blocks Mario cannot be inside (0:1EB4 `cp $38`).
pub const fn solid(block: u8) -> bool {
    block < 0x38
}

/// Blocks Mario can stand on (0:1E60 `cp $48`: solid ones and platforms).
pub const fn ground(block: u8) -> bool {
    block < 0x48
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_00_header() {
        let h = [0xD4, 0x01, 0x22, 0x00, 0xC0, 0x01, 0x50, 0x00, 0xAA];
        let s = Start::read(&h);
        assert_eq!(s, Start { mario_x: 0x22, mario_y: 0x1D4, camera_x: 0x50, camera_y: 0x1C0 });
        let mut back = [0u8; 9];
        back[8] = 0xAA;
        s.write(&mut back);
        assert_eq!(back, h);
        assert_eq!(Start::standing(0x22, 0x1F0 + 7), s);
    }

    #[test]
    fn a_body_in_a_wall_is_blocked() {
        let mut map = vec![0x60u8; COLS * ROWS];
        let s = Start::standing(0x100, 0x1F0);
        assert!(!s.blocked(&map));
        map[(0x1F0 / 16 - 1) * COLS + 0x100 / 16] = 0x10;
        assert!(s.blocked(&map));
        assert!(Start::standing(4090, 0x100).blocked(&[0x60u8; COLS * ROWS]) == false);
    }

    #[test]
    fn the_objects_passed_at_the_start_are_those_the_game_skipped() {
        // research `startobjects`: the last x skipped and the first that came to life
        let s = |camera_x| Start { camera_x, ..Start::default() };
        assert!(s(80).passed_at_start(184) && !s(80).passed_at_start(192));
        assert!(s(88).passed_at_start(192) && !s(87).passed_at_start(192));
        assert!(s(465).passed_at_start(352) && !s(465).passed_at_start(344));
        assert!(!s(466).passed_at_start(352));
        assert!(s(80).passed_at_start(0));
    }

    #[test]
    fn camera_stays_inside_the_level() {
        let right = Start::standing(4090, 0x100);
        assert_eq!((right.mario_x, right.camera_x), (4096 - 16, 4096 - 0x50));
        let top = Start::standing(0x900, 0);
        assert_eq!((top.mario_y, top.camera_y), (4, CAM_Y_MIN));
        let bottom = Start::standing(0x900, 2000);
        assert_eq!((bottom.mario_y + FEET, bottom.camera_y), (768, CAM_Y_MAX));
        let mid = Start::standing(0x929, 0x2F0);
        assert_eq!((mid.mario_y, mid.camera_x, mid.camera_y), (0x2D4, 0x94A, 0x2C0));
    }
}
