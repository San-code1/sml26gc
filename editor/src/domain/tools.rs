//! Tools on a grid (level blocks or map tiles). A tool gets the mouse button's state, the cell
//! under it and the stamp to paint each frame, and paints through `GridCanvas`; a new tool is
//! a new `Tool` implementation, nothing else changes.
use super::stamp::{Area, Stamp};

/// The left mouse button during one frame.
#[derive(Clone, Copy, Default)]
pub struct Pointer {
    pub pressed: bool,
    pub down: bool,
    pub released: bool,
}

/// What a tool paints on. Cells outside the grid are ignored.
pub trait GridCanvas {
    /// Called once before each change, so it can be undone.
    fn begin_edit(&mut self);
    fn paint(&mut self, x: i32, y: i32, v: u8);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToolKind {
    Pencil,
    Box,
    Select,
    Paste,
}

pub trait Tool {
    fn kind(&self) -> ToolKind;
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer, stamp: &Stamp);
    /// The corner where a rectangle started, while one is being dragged.
    fn anchor(&self) -> Option<(i32, i32)> {
        None
    }
    /// The rectangle selected (Select only).
    fn selection(&self) -> Option<Area> {
        None
    }
    fn clear_selection(&mut self) {}
}

pub fn make(kind: ToolKind) -> Box<dyn Tool> {
    match kind {
        ToolKind::Pencil => Box::new(Pencil::default()),
        ToolKind::Box => Box::new(BoxFill::default()),
        ToolKind::Select => Box::new(Select::default()),
        ToolKind::Paste => Box::new(Paste),
    }
}

/// The stamp with its top-left at `(x, y)`, its pattern counted from `origin`.
fn stamp_at(canvas: &mut dyn GridCanvas, stamp: &Stamp, (x, y): (i32, i32), origin: (i32, i32)) {
    for dy in 0..stamp.h as i32 {
        for dx in 0..stamp.w as i32 {
            canvas.paint(x + dx, y + dy, stamp.at(x + dx - origin.0, y + dy - origin.1));
        }
    }
}

/// Paints the stamp wherever the mouse passes while held; the pattern is counted from where
/// the stroke started, so it repeats without seams.
#[derive(Default)]
pub struct Pencil {
    origin: Option<(i32, i32)>,
    last: Option<(i32, i32)>,
}

impl Tool for Pencil {
    fn kind(&self) -> ToolKind {
        ToolKind::Pencil
    }
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer, stamp: &Stamp) {
        if !p.down {
            self.origin = None;
            self.last = None;
            return;
        }
        let Some(c) = cell else { return };
        let origin = *self.origin.get_or_insert_with(|| {
            canvas.begin_edit();
            c
        });
        if self.last != Some(c) {
            self.last = Some(c);
            stamp_at(canvas, stamp, c, origin);
        }
    }
}

/// Fills the rectangle between where the button went down and where it came up with the
/// stamp, repeated from the rectangle's top-left.
#[derive(Default)]
pub struct BoxFill {
    start: Option<(i32, i32)>,
}

impl Tool for BoxFill {
    fn kind(&self) -> ToolKind {
        ToolKind::Box
    }
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer, stamp: &Stamp) {
        if p.pressed && cell.is_some() {
            self.start = cell;
        }
        if p.released {
            if let (Some(s), Some(e)) = (self.start.take(), cell) {
                canvas.begin_edit();
                let a = Area::between(s, e);
                for (x, y) in a.cells() {
                    canvas.paint(x, y, stamp.at(x - a.x0, y - a.y0));
                }
            }
        }
    }
    fn anchor(&self) -> Option<(i32, i32)> {
        self.start
    }
}

/// Marks a rectangle; paints nothing.
#[derive(Default)]
pub struct Select {
    start: Option<(i32, i32)>,
    end: (i32, i32),
    chosen: Option<Area>,
}

impl Tool for Select {
    fn kind(&self) -> ToolKind {
        ToolKind::Select
    }
    fn handle(&mut self, _: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer, _: &Stamp) {
        if p.pressed {
            if let Some(c) = cell {
                self.start = Some(c);
                self.end = c;
            }
        }
        if let (Some(s), Some(c)) = (self.start, cell) {
            self.end = c;
            self.chosen = Some(Area::between(s, c));
        }
        if p.released {
            self.start = None;
        }
    }
    fn anchor(&self) -> Option<(i32, i32)> {
        self.start
    }
    fn selection(&self) -> Option<Area> {
        self.chosen
    }
    fn clear_selection(&mut self) {
        self.start = None;
        self.chosen = None;
    }
}

/// Each click places the stamp (the clipboard) with its top-left under the mouse.
pub struct Paste;

impl Tool for Paste {
    fn kind(&self) -> ToolKind {
        ToolKind::Paste
    }
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer, stamp: &Stamp) {
        if let (true, Some(c)) = (p.pressed, cell) {
            canvas.begin_edit();
            stamp_at(canvas, stamp, c, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Log {
        edits: u32,
        painted: Vec<(i32, i32)>,
        cells: HashMap<(i32, i32), u8>,
    }
    impl GridCanvas for Log {
        fn begin_edit(&mut self) {
            self.edits += 1;
        }
        fn paint(&mut self, x: i32, y: i32, v: u8) {
            self.painted.push((x, y));
            self.cells.insert((x, y), v);
        }
    }

    const PRESS: Pointer = Pointer { pressed: true, down: true, released: false };
    const HOLD: Pointer = Pointer { pressed: false, down: true, released: false };
    const RELEASE: Pointer = Pointer { pressed: false, down: false, released: true };

    fn two_by_two() -> Stamp {
        Stamp { w: 2, h: 2, cells: vec![1, 2, 3, 4] }
    }

    #[test]
    fn pencil_one_edit_per_stroke() {
        let (mut t, mut c, s) = (Pencil::default(), Log::default(), Stamp::single(9));
        t.handle(&mut c, Some((1, 1)), PRESS, &s);
        t.handle(&mut c, Some((2, 1)), HOLD, &s);
        t.handle(&mut c, Some((2, 1)), HOLD, &s);
        t.handle(&mut c, None, HOLD, &s);
        t.handle(&mut c, Some((3, 1)), RELEASE, &s);
        assert_eq!(c.edits, 1);
        assert_eq!(c.painted, vec![(1, 1), (2, 1)]);
    }

    #[test]
    fn pencil_paints_the_whole_stamp_without_seams() {
        let (mut t, mut c) = (Pencil::default(), Log::default());
        t.handle(&mut c, Some((10, 4)), PRESS, &two_by_two());
        assert_eq!([c.cells[&(10, 4)], c.cells[&(11, 4)], c.cells[&(10, 5)], c.cells[&(11, 5)]], [1, 2, 3, 4]);
        t.handle(&mut c, Some((11, 4)), HOLD, &two_by_two());
        assert_eq!([c.cells[&(11, 4)], c.cells[&(12, 4)]], [2, 1]);
    }

    #[test]
    fn box_repeats_the_stamp() {
        let (mut t, mut c) = (BoxFill::default(), Log::default());
        let s = Stamp { w: 2, h: 1, cells: vec![7, 8] };
        t.handle(&mut c, Some((0, 2)), PRESS, &s);
        assert_eq!(t.anchor(), Some((0, 2)));
        t.handle(&mut c, Some((4, 2)), HOLD, &s);
        assert!(c.painted.is_empty());
        t.handle(&mut c, Some((4, 2)), RELEASE, &s);
        let row: Vec<u8> = (0..5).map(|x| c.cells[&(x, 2)]).collect();
        assert_eq!((c.edits, row, t.anchor()), (1, vec![7, 8, 7, 8, 7], None));
    }

    #[test]
    fn select_marks_and_paints_nothing() {
        let (mut t, mut c, s) = (Select::default(), Log::default(), Stamp::single(0));
        t.handle(&mut c, Some((5, 4)), PRESS, &s);
        t.handle(&mut c, Some((2, 3)), HOLD, &s);
        t.handle(&mut c, Some((2, 3)), RELEASE, &s);
        assert_eq!(t.selection(), Some(Area::between((2, 3), (5, 4))));
        assert_eq!((c.edits, c.painted.len()), (0, 0));
        t.clear_selection();
        assert_eq!(t.selection(), None);
    }

    #[test]
    fn each_paste_click_is_one_edit() {
        let (mut t, mut c) = (Paste, Log::default());
        t.handle(&mut c, Some((10, 5)), PRESS, &two_by_two());
        t.handle(&mut c, Some((11, 5)), HOLD, &two_by_two());
        t.handle(&mut c, Some((11, 5)), RELEASE, &two_by_two());
        t.handle(&mut c, Some((20, 5)), PRESS, &two_by_two());
        assert_eq!((c.edits, c.painted.len()), (2, 8));
        assert_eq!(c.cells[&(21, 6)], 4);
    }
}
