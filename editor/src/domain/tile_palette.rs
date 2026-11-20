//! The tile and block palettes' layout: tiles shown as the game draws them. A zone's palette is
//! its picture; the world map's is ready-made pieces of its picture, picked by hand from it
//! (`PIECES`), side by side. Under them come the tiles they leave out. A level's block palette
//! is its 128 blocks in order. A selection stays in the group it starts in (a piece, the
//! picture, the rest).

/// Cells per row of a map's palette.
pub const COLS: usize = 20;
/// Cells per row of a level's block palette.
pub const BLOCK_COLS: usize = 8;

/// A piece of the game's world map picture (64 x 64 tiles): its name and place in tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub name: &'static str,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

const fn piece(name: &'static str, x: usize, y: usize, w: usize, h: usize) -> Piece {
    Piece { name, x, y, w, h }
}

/// The world map's pieces, largest first.
pub const PIECES: [Piece; 12] = [
    piece("Pumpkin Zone", 39, 10, 19, 12),
    piece("Wario's castle", 25, 20, 14, 8),
    piece("Tree Zone tree", 6, 34, 12, 7),
    piece("Star and moon", 31, 8, 8, 4),
    piece("Hill with a door", 33, 41, 7, 4),
    piece("Macro Zone house", 24, 13, 5, 5),
    piece("Mushroom house", 23, 45, 5, 5),
    piece("Turtle Zone shell", 52, 45, 5, 5),
    piece("Mario Zone", 48, 27, 4, 8),
    piece("Bridge", 8, 25, 4, 7),
    piece("Ladder", 7, 17, 1, 4),
    piece("Pipe", 39, 16, 2, 2),
];

/// The cells, `cols` per row: a tile (or block) or nothing, and the group each belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileCells {
    pub cols: usize,
    pub cells: Vec<Option<u8>>,
    pub groups: Vec<Option<usize>>,
    /// Each group's name (the pieces'; empty for a picture and the tiles left out).
    pub names: Vec<&'static str>,
    /// The first row of the tiles left out, after an empty row (where a rule is drawn).
    pub rest_row: usize,
}

impl TileCells {
    /// A zone's palette: its picture (`w` tiles per row), then the tiles it leaves out.
    pub fn zone(picture: &[u8], w: usize) -> TileCells {
        let mut t = TileCells { cols: COLS, cells: Vec::new(), groups: Vec::new(), names: vec![""], rest_row: 0 };
        for row in picture.chunks(w) {
            for c in 0..COLS {
                t.cells.push(row.get(c).copied());
                t.groups.push(row.get(c).map(|_| 0));
            }
        }
        t.add_rest(picture, 256);
        t
    }

    /// The world map's palette: `PIECES` of `picture` (64 x 64 tiles) side by side, a row apart,
    /// then the tiles they leave out.
    pub fn world(picture: &[u8]) -> TileCells {
        let pieces: Vec<(Vec<u8>, usize, usize)> = PIECES.iter().map(|p| ((0..p.h).flat_map(|r| (0..p.w).map(move |c| picture[(p.y + r) * 64 + p.x + c])).collect(), p.w, p.h)).collect();
        let mut t = TileCells { cols: COLS, cells: Vec::new(), groups: Vec::new(), names: PIECES.iter().map(|p| p.name).collect(), rest_row: 0 };
        t.pack(&pieces);
        let used: Vec<u8> = pieces.iter().flat_map(|p| p.0.iter().copied()).collect();
        t.add_rest(&used, 256);
        t
    }

    /// A level's block palette: its 128 blocks in order.
    pub fn blocks() -> TileCells {
        let mut t = TileCells { cols: BLOCK_COLS, cells: Vec::new(), groups: Vec::new(), names: Vec::new(), rest_row: 0 };
        t.add_rest(&[], 128);
        t
    }

    /// `pieces` (cells row by row, width, height) side by side, a cell apart, as many rows as
    /// they need; each its own group, in order.
    fn pack(&mut self, pieces: &[(Vec<u8>, usize, usize)]) {
        let (mut x, mut shelf, mut tall) = (0, 0, 0);
        for (g, (cells, w, h)) in pieces.iter().enumerate() {
            if x > 0 && x + w > self.cols {
                (x, shelf, tall) = (0, shelf + tall + 1, 0);
            }
            for r in 0..*h {
                for c in 0..*w {
                    self.put(x + c, shelf + r, cells[r * w + c], g);
                }
            }
            x += w + 1;
            tall = tall.max(*h);
        }
        self.cells.resize((shelf + tall) * self.cols, None);
        self.groups.resize(self.cells.len(), None);
    }

    fn put(&mut self, x: usize, y: usize, v: u8, g: usize) {
        let i = y * self.cols + x;
        if self.cells.len() <= i {
            self.cells.resize((y + 1) * self.cols, None);
            self.groups.resize((y + 1) * self.cols, None);
        }
        self.cells[i] = Some(v);
        self.groups[i] = Some(g);
    }

    /// The values below `count` not in `used`, in order, as their own group after an empty row
    /// (none when nothing comes before them).
    fn add_rest(&mut self, used: &[u8], count: usize) {
        if !self.cells.is_empty() {
            self.cells.resize(self.cells.len() + self.cols, None);
            self.groups.resize(self.cells.len(), None);
        }
        self.rest_row = self.cells.len() / self.cols;
        let g = self.names.len();
        self.names.push("");
        for v in (0..count).map(|v| v as u8).filter(|v| !used.contains(v)) {
            self.cells.push(Some(v));
            self.groups.push(Some(g));
        }
    }

    /// The cell showing `v` (column, row): in the rest when it is there, else the first.
    pub fn cell_of(&self, v: u8) -> Option<(i32, i32)> {
        let rest = self.rest_row * self.cols;
        let i = self.cells[rest.min(self.cells.len())..].iter().position(|&c| c == Some(v)).map(|i| i + rest).or_else(|| self.cells.iter().position(|&c| c == Some(v)))?;
        Some(((i % self.cols) as i32, (i / self.cols) as i32))
    }

    pub fn at(&self, (c, r): (i32, i32)) -> Option<usize> {
        let i = r as usize * self.cols + c as usize;
        (c >= 0 && r >= 0 && (c as usize) < self.cols && i < self.cells.len()).then_some(i)
    }

    /// The group of cell (column, row).
    pub fn group(&self, cell: (i32, i32)) -> Option<usize> {
        self.groups[self.at(cell)?]
    }

    /// A drag from `from` to `to`: the rectangle of cells inside `from`'s group (its corners)
    /// and their tiles, row by row; a single cell when the rectangle has empty ones.
    pub fn select(&self, from: (i32, i32), to: (i32, i32)) -> Option<((i32, i32), (i32, i32), Vec<u8>)> {
        let g = self.group(from)?;
        let mine: Vec<(i32, i32)> = (0..self.cells.len()).filter(|&i| self.groups[i] == Some(g)).map(|i| ((i % self.cols) as i32, (i / self.cols) as i32)).collect();
        let (x0, x1) = (mine.iter().map(|c| c.0).min()?, mine.iter().map(|c| c.0).max()?);
        let (y0, y1) = (mine.iter().map(|c| c.1).min()?, mine.iter().map(|c| c.1).max()?);
        let (a, b) = ((from.0.min(to.0).clamp(x0, x1), from.1.min(to.1).clamp(y0, y1)), (from.0.max(to.0).clamp(x0, x1), from.1.max(to.1).clamp(y0, y1)));
        let tiles: Option<Vec<u8>> = (a.1..=b.1).flat_map(|r| (a.0..=b.0).map(move |c| (c, r))).map(|c| self.at(c).and_then(|i| self.cells[i]).filter(|_| self.group(c) == Some(g))).collect();
        match tiles {
            Some(t) => Some((a, b, t)),
            None => Some((from, from, vec![self.cells[self.at(from)?]?])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zone_is_its_picture_then_the_tiles_it_leaves_out() {
        let picture: Vec<u8> = (0..6).collect(); // 3 x 2
        let t = TileCells::zone(&picture, 3);
        assert_eq!(t.rest_row, 3);
        assert_eq!((t.cells[COLS], t.cells[3]), (Some(3), None));
        assert_eq!(t.cell_of(6), Some((0, 3)), "the first tile left out");
        assert_eq!(t.select((0, 0), (9, 9)), Some(((0, 0), (2, 1), vec![0, 1, 2, 3, 4, 5])), "a drag past the picture takes all of it");
    }

    #[test]
    fn the_world_pieces_are_side_by_side_and_a_drag_stays_in_one() {
        let picture: Vec<u8> = (0..64 * 64).map(|i| (i % 251) as u8).collect();
        let t = TileCells::world(&picture);
        // the first piece, the Pumpkin Zone, from its top left
        let p = PIECES[0];
        let (a, b, tiles) = t.select((0, 0), (40, 40)).unwrap();
        assert_eq!((a, b), ((0, 0), (p.w as i32 - 1, p.h as i32 - 1)));
        assert_eq!(tiles[0], picture[p.y * 64 + p.x]);
        assert!(PIECES.iter().all(|p| p.w <= COLS && p.x + p.w <= 64 && p.y + p.h <= 64));
        let rest = t.rest_row as i32;
        assert_eq!(t.group((0, rest)), Some(PIECES.len()), "the tiles left out");
    }

    #[test]
    fn a_level_palette_is_its_128_blocks_in_order() {
        let t = TileCells::blocks();
        assert_eq!((t.cols, t.rest_row, t.cells.len()), (BLOCK_COLS, 0, 128));
        assert_eq!(t.cell_of(0x21), Some((1, 4)));
        assert_eq!(t.select((0, 0), (1, 1)).map(|s| s.2), Some(vec![0x00, 0x01, 0x08, 0x09]));
    }
}
