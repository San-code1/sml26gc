//! An empty map made from a game level's: its sky everywhere, a floor of the block Mario
//! stands on at its start from that row to the bottom, and Mario's start on it near the left.
use crate::level_start::{ground, Start};
use crate::levels::{COLS, ROWS};

/// The column Mario starts on in a blank level.
const START_COL: usize = 2;

/// The level's sky: the block it uses most among those Mario cannot stand on.
pub fn sky(map: &[u8]) -> u8 {
    let mut count = [0usize; 128];
    map.iter().filter(|&&b| !ground(b & 0x7F)).for_each(|&b| count[(b & 0x7F) as usize] += 1);
    (0..128).max_by_key(|&b| count[b]).unwrap_or(0) as u8
}

/// The block under Mario's feet at `start` (or the first one he could stand on below them)
/// and its row; None when there is none down to the bottom.
fn floor(map: &[u8], start: &Start) -> Option<(u8, usize)> {
    let col = ((start.mario_x + 8) / 16) as usize;
    let first = ((start.mario_y + crate::level_start::FEET) / 16) as usize;
    (first..ROWS).map(|r| (map[r * COLS + col.min(COLS - 1)] & 0x7F, r)).find(|&(b, _)| ground(b))
}

/// The blank map and Mario's start in it, from game level map `map` and its start.
pub fn blank(map: &[u8], start: &Start) -> (Vec<u8>, Start) {
    let sky = sky(map);
    let (block, row) = floor(map, start).unwrap_or((0x00, ROWS - 2));
    let mut out = vec![sky; COLS * ROWS];
    out[row * COLS..].fill(block);
    (out, Start::standing((START_COL * 16) as u16, (row * 16) as u16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mario_stands_on_the_floor_of_an_empty_sky() {
        let mut map = vec![0x60u8; COLS * ROWS];
        map[..COLS].fill(0x70);
        for c in 0..COLS {
            map[31 * COLS + c] = 0x12;
            map[32 * COLS + c] = 0x13;
        }
        let start = Start::standing(0x22, 0x1F0 + 7);
        let (m, s) = blank(&map, &start);
        assert_eq!(sky(&map), 0x60);
        assert!(m[..31 * COLS].iter().all(|&b| b == 0x60));
        assert!(m[31 * COLS..].iter().all(|&b| b == 0x12));
        assert!(!s.blocked(&m), "his body is in the sky");
        assert_eq!((s.mario_x, (s.mario_y + crate::level_start::FEET) / 16), (32, 31), "his feet on the floor");
    }

    #[test]
    fn a_start_in_the_air_finds_the_ground_below() {
        let mut map = vec![0x60u8; COLS * ROWS];
        for c in 0..COLS {
            map[40 * COLS + c] = 0x20;
        }
        let (m, s) = blank(&map, &Start::standing(0x22, 0x1F0));
        assert_eq!(m[40 * COLS], 0x20);
        assert_eq!((s.mario_y + crate::level_start::FEET) / 16, 40);
    }
}
