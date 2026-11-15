//! Levels as the cartridge stores them, read from its code (see MECHANICS.md, "Levels"):
//!
//! * 32 level headers of 20 bytes, pointed to by the word table at bank 1 $55CB
//!   (indexed by the level number at $A269, copied to $A800 by 01:5579).
//!   Header byte $0D = ROM bank of the level map, low nibble of byte $12 = map number,
//!   bytes $0B-$0C = address (bank 8) of the 128 block definitions copied to $A600 (0:03FF).
//! * In the map bank, the word table at $42F0 gives each map's compressed data (0:0361).
//! * Compression (0:0361): a byte below $80 is one block; $80+v followed by n writes block v
//!   1+n times (n = 0 means 256). Runs stop at the end of each 256-block row. The decoder
//!   fills $B000-$DFFF: 48 rows of 256 blocks, each block 16x16 pixels.
//! * A block is four 8x8 tiles (top-left, top-right, bottom-left, bottom-right), $A600 + 4*id.
use crate::patch::data_bank;
use crate::rom::{offset, word};

pub const LEVELS: usize = 32;
pub const COLS: usize = 256;
pub const ROWS: usize = 48;
pub const MAP_BYTES: usize = COLS * ROWS;
const HEADER_TABLE: usize = 0x55CB; // bank 1, file offset = address
pub(crate) const MAP_TABLE: u16 = 0x42F0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Header {
    /// File offset of the 20-byte header.
    pub at: usize,
    pub bank: u8,
    pub map: u8,
}

pub fn header(rom: &[u8], level: usize) -> Header {
    let at = offset(1, word(rom, HEADER_TABLE + level * 2));
    Header { at, bank: rom[at + 0x0D], map: rom[at + 0x12] & 0x0F }
}

/// File offset of the map table entry and of the compressed data.
pub fn map_location(rom: &[u8], h: Header) -> (usize, usize) {
    let entry = offset(h.bank as usize, MAP_TABLE) + h.map as usize * 2;
    (entry, offset(data_bank(rom, h.bank) as usize, word(rom, entry)))
}

/// Decode a map exactly as 0:0361 does; returns the blocks and how many bytes it read.
pub fn decode(rom: &[u8], start: usize) -> (Vec<u8>, usize) {
    let mut out = vec![0u8; MAP_BYTES];
    let mut pos = 0usize;
    let mut src = start;
    let next = |src: &mut usize| -> u8 {
        let b = rom.get(*src).copied().unwrap_or(0);
        *src += 1;
        b
    };
    while pos < MAP_BYTES {
        let b = next(&mut src);
        if b & 0x80 == 0 {
            out[pos] = b;
            pos += 1;
            continue;
        }
        let v = b & 0x7F;
        out[pos] = v;
        pos += 1;
        if pos % COLS == 0 {
            continue;
        }
        let mut n = next(&mut src);
        loop {
            out[pos] = v;
            pos += 1;
            if pos % COLS == 0 {
                break;
            }
            n = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
        }
    }
    (out, src - start)
}

/// Compress a map so that `decode` gives it back exactly.
pub fn encode(map: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for row in map.chunks(COLS) {
        let mut i = 0;
        while i < row.len() {
            let v = row[i] & 0x7F;
            let mut len = 1;
            while i + len < row.len() && row[i + len] & 0x7F == v && len < 256 {
                len += 1;
            }
            if len == 1 {
                out.push(v);
            } else {
                out.push(0x80 | v);
                out.push((len - 1) as u8);
            }
            i += len;
        }
    }
    out
}

/// Levels that use the same map (bank and number) as `level`.
pub fn sharing(rom: &[u8], level: usize) -> Vec<usize> {
    let h = header(rom, level);
    (0..LEVELS)
        .filter(|&l| {
            l != level && {
                let o = header(rom, l);
                o.bank == h.bank && o.map == h.map
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(map: &[u8]) -> Vec<u8> {
        let enc = encode(map);
        let (dec, used) = decode(&enc, 0);
        assert_eq!(used, enc.len(), "decoder must read exactly what the encoder wrote");
        dec
    }

    #[test]
    fn empty_and_uniform_maps() {
        let zeros = vec![0u8; MAP_BYTES];
        assert_eq!(round_trip(&zeros), zeros);
        // a whole row of one block is one run of 256 (count byte 255)
        assert_eq!(&encode(&zeros)[..2], &[0x80, 0xFF]);
        assert_eq!(encode(&zeros).len(), ROWS * 2);
    }

    #[test]
    fn varied_maps_round_trip() {
        let mut m = vec![0u8; MAP_BYTES];
        for (i, b) in m.iter_mut().enumerate() {
            // runs of different lengths, crossing nothing but row ends
            *b = ((i / 7) % 128) as u8;
        }
        assert_eq!(round_trip(&m), m);
        let mut alt = vec![0u8; MAP_BYTES];
        for (i, b) in alt.iter_mut().enumerate() {
            *b = (i % 2) as u8 * 0x7F;
        }
        assert_eq!(round_trip(&alt), alt);
    }

    #[test]
    fn runs_stop_at_row_ends() {
        // the last block of a row followed by the same block on the next row:
        // two separate runs, never one crossing the row end
        let mut m = vec![1u8; MAP_BYTES];
        m[COLS - 1] = 5;
        m[COLS] = 5;
        assert_eq!(round_trip(&m), m);
    }

    #[test]
    fn decoder_run_of_zero_means_256() {
        // $80+v with n = 0: block v until the end of the row (256 at most)
        let mut data = Vec::new();
        for _ in 0..ROWS {
            data.extend_from_slice(&[0x83, 0x00]);
        }
        let (map, used) = decode(&data, 0);
        assert!(map.iter().all(|&b| b == 3));
        assert_eq!(used, data.len());
    }

    #[test]
    fn high_bit_of_blocks_is_ignored_by_encode() {
        let m = vec![0x85u8; MAP_BYTES];
        assert!(round_trip(&m).iter().all(|&b| b == 0x05));
    }
}
