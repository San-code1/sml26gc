//! A rectangle of block or tile numbers: the palette selection, a piece picked from the map,
//! or the clipboard.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub w: usize,
    pub h: usize,
    /// Row by row.
    pub cells: Vec<u8>,
}

impl Stamp {
    pub fn single(v: u8) -> Stamp {
        Stamp { w: 1, h: 1, cells: vec![v] }
    }

    /// The value at `(dx, dy)` from its top-left, repeating it in every direction.
    pub fn at(&self, dx: i32, dy: i32) -> u8 {
        let x = dx.rem_euclid(self.w as i32) as usize;
        let y = dy.rem_euclid(self.h as i32) as usize;
        self.cells[y * self.w + x]
    }

    pub fn is_single(&self) -> bool {
        self.w == 1 && self.h == 1
    }
}

/// Cells of a grid between two corners, both included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Area {
    pub fn between(a: (i32, i32), b: (i32, i32)) -> Area {
        Area { x0: a.0.min(b.0), y0: a.1.min(b.1), x1: a.0.max(b.0), y1: a.1.max(b.1) }
    }

    pub fn w(&self) -> usize {
        (self.x1 - self.x0 + 1) as usize
    }

    pub fn h(&self) -> usize {
        (self.y1 - self.y0 + 1) as usize
    }

    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.y0..=self.y1).flat_map(move |y| (self.x0..=self.x1).map(move |x| (x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_repeats_in_every_direction() {
        let s = Stamp { w: 2, h: 2, cells: vec![1, 2, 3, 4] };
        assert_eq!((s.at(0, 0), s.at(1, 0), s.at(0, 1), s.at(1, 1)), (1, 2, 3, 4));
        assert_eq!((s.at(2, 0), s.at(-1, 0), s.at(0, -1), s.at(3, 3)), (1, 2, 3, 4));
    }

    #[test]
    fn single_and_area() {
        let s = Stamp::single(7);
        assert!(s.is_single());
        assert_eq!(s.at(5, -3), 7);
        let a = Area::between((5, 4), (2, 3));
        assert_eq!((a.x0, a.y0, a.w(), a.h(), a.cells().count()), (2, 3, 4, 2, 8));
    }
}
