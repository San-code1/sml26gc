//! Painting tools on a grid (level blocks or world map tiles). A tool gets the mouse
//! button's state and the cell under it each frame and paints through `GridCanvas`; a new
//! tool is a new `Tool` implementation, nothing else changes.

/// The left mouse button during one frame.
#[derive(Clone, Copy, Default)]
pub struct Pointer {
    pub pressed: bool,
    pub down: bool,
    pub released: bool,
}

/// What a tool paints on.
pub trait GridCanvas {
    /// Called once before each change, so it can be undone.
    fn begin_edit(&mut self);
    fn paint(&mut self, x: i32, y: i32);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToolKind {
    Pencil,
    Box,
}

pub trait Tool {
    fn kind(&self) -> ToolKind;
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer);
    /// The corner where a box started, while one is being dragged.
    fn anchor(&self) -> Option<(i32, i32)> {
        None
    }
}

pub fn make(kind: ToolKind) -> Box<dyn Tool> {
    match kind {
        ToolKind::Pencil => Box::new(Pencil),
        ToolKind::Box => Box::new(BoxFill::default()),
    }
}

/// Paints every cell the mouse passes over while held.
pub struct Pencil;

impl Tool for Pencil {
    fn kind(&self) -> ToolKind {
        ToolKind::Pencil
    }
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer) {
        if p.pressed && cell.is_some() {
            canvas.begin_edit();
        }
        if p.down {
            if let Some((x, y)) = cell {
                canvas.paint(x, y);
            }
        }
    }
}

/// Fills the rectangle between where the button went down and where it came up.
#[derive(Default)]
pub struct BoxFill {
    start: Option<(i32, i32)>,
}

impl Tool for BoxFill {
    fn kind(&self) -> ToolKind {
        ToolKind::Box
    }
    fn handle(&mut self, canvas: &mut dyn GridCanvas, cell: Option<(i32, i32)>, p: Pointer) {
        if p.pressed && cell.is_some() {
            self.start = cell;
        }
        if p.released {
            if let (Some((x0, y0)), Some((x1, y1))) = (self.start.take(), cell) {
                canvas.begin_edit();
                for y in y0.min(y1)..=y0.max(y1) {
                    for x in x0.min(x1)..=x0.max(x1) {
                        canvas.paint(x, y);
                    }
                }
            }
        }
    }
    fn anchor(&self) -> Option<(i32, i32)> {
        self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Log {
        edits: u32,
        painted: Vec<(i32, i32)>,
    }
    impl GridCanvas for Log {
        fn begin_edit(&mut self) {
            self.edits += 1;
        }
        fn paint(&mut self, x: i32, y: i32) {
            self.painted.push((x, y));
        }
    }

    const PRESS: Pointer = Pointer {
        pressed: true,
        down: true,
        released: false,
    };
    const HOLD: Pointer = Pointer {
        pressed: false,
        down: true,
        released: false,
    };
    const RELEASE: Pointer = Pointer {
        pressed: false,
        down: false,
        released: true,
    };

    #[test]
    fn pencil_one_edit_per_stroke() {
        let (mut t, mut c) = (Pencil, Log::default());
        t.handle(&mut c, Some((1, 1)), PRESS);
        t.handle(&mut c, Some((2, 1)), HOLD);
        t.handle(&mut c, None, HOLD);
        t.handle(&mut c, Some((3, 1)), RELEASE);
        assert_eq!(c.edits, 1);
        assert_eq!(c.painted, vec![(1, 1), (2, 1)]);
    }

    #[test]
    fn box_fills_on_release() {
        let (mut t, mut c) = (BoxFill::default(), Log::default());
        t.handle(&mut c, Some((3, 2)), PRESS);
        assert_eq!(t.anchor(), Some((3, 2)));
        t.handle(&mut c, Some((2, 3)), HOLD);
        assert!(c.painted.is_empty());
        t.handle(&mut c, Some((2, 3)), RELEASE);
        assert_eq!((c.edits, c.painted.len(), t.anchor()), (1, 4, None));
    }
}
