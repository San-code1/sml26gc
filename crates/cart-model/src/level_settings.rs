//! A level's settings kept in its 20-byte header (research `headerreads`): the start time,
//! the music and the background shades.
//! - $13: start time in hundreds (0:04A4: $A255 = it, $A254 = 0).
//! - $0E: music (0:0592 puts it in $A468, the music request).
//! - $0F: background palette (0:0503 writes it to $FF47); $10-$11 are the sprite palettes.

const TIME: usize = 0x13;
const MUSIC: usize = 0x0E;
const SHADES: usize = 0x0F;

/// The musics the game's levels use, each with the first level that plays it.
pub const MUSICS: [(u8, usize); 9] = [(0x01, 0), (0x14, 1), (0x11, 2), (0x28, 3), (0x1F, 8), (0x17, 14), (0x13, 18), (0x1D, 19), (0x0B, 24)];
/// The background palettes the game's levels use: normal, then darker.
pub const PALETTES: [u8; 3] = [0xE4, 0xE1, 0x93];
/// Start time limits, in hundreds.
pub const TIME_MIN: u8 = 1;
pub const TIME_MAX: u8 = 9;

/// A level's settings as its header keeps them (unknown values are kept as they are).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Settings {
    /// Hundreds.
    pub time: u8,
    pub music: u8,
    /// The background palette byte.
    pub palette: u8,
}

impl Settings {
    pub fn read(header: &[u8]) -> Settings {
        Settings { time: header[TIME], music: header[MUSIC], palette: header[SHADES] }
    }

    pub fn write(&self, header: &mut [u8]) {
        header[TIME] = self.time;
        header[MUSIC] = self.music;
        header[SHADES] = self.palette;
    }
}

/// Start time in hundreds.
pub fn time(header: &[u8]) -> u8 {
    header[TIME]
}

pub fn set_time(header: &mut [u8], hundreds: u8) {
    header[TIME] = hundreds.clamp(TIME_MIN, TIME_MAX);
}

pub fn music(header: &[u8]) -> u8 {
    header[MUSIC]
}

pub fn set_music(header: &mut [u8], music: u8) {
    header[MUSIC] = music;
}

/// Index into `PALETTES` (an unknown value reads as normal).
pub fn shades(header: &[u8]) -> usize {
    PALETTES.iter().position(|&p| p == header[SHADES]).unwrap_or(0)
}

pub fn set_shades(header: &mut [u8], index: usize) {
    header[SHADES] = PALETTES[index.min(PALETTES.len() - 1)];
}

/// Index into `MUSICS` (an unknown value reads as the first).
pub fn music_index(header: &[u8]) -> usize {
    MUSICS.iter().position(|&(m, _)| m == header[MUSIC]).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_in_the_header() {
        // level 1's header
        let mut h = [0xD4, 0x01, 0x22, 0x00, 0xC0, 0x01, 0x50, 0x00, 0x80, 0x5B, 0x01, 0x00, 0x42, 0x0A, 0x14, 0xE4, 0xD0, 0x38, 0x00, 0x05];
        assert_eq!((time(&h), music(&h), shades(&h), music_index(&h)), (5, 0x14, 0, 1));
        set_time(&mut h, 12);
        set_music(&mut h, 0x28);
        set_shades(&mut h, 2);
        assert_eq!((time(&h), music_index(&h), shades(&h), h[0x0F]), (9, 3, 2, 0x93));
    }
}
