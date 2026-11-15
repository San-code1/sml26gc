//! Painting a level's blocks or a map picture's tiles, the same way in both views: the stamp,
//! picking one from the grid with the right button, the selection, the clipboard and the
//! previews drawn over the grid.
use super::widgets::*;
use super::{left_button, DocCanvas, Editor};
use crate::domain::document::Grid;
use crate::domain::stamp::{Area, Stamp};
use crate::domain::tools::ToolKind;
use macroquad::prelude::*;

/// What a palette selection or a pick from the grid gives to paint with.
pub struct Brush {
    pub stamp: Stamp,
    /// The palette cells it came from (columns and rows of the palette), to outline them.
    pub in_palette: Option<Area>,
}

impl Brush {
    pub fn new() -> Brush {
        Brush { stamp: Stamp::single(0), in_palette: Some(Area::between((0, 0), (0, 0))) }
    }

    /// The first value of the stamp (the one named in the panel).
    pub fn first(&self) -> u8 {
        self.stamp.cells[0]
    }
}

impl Editor {
    /// The grid shown.
    pub(super) fn grid(&self) -> Grid {
        if self.in_world {
            Grid::Map(self.world_map)
        } else {
            Grid::Level(self.cur)
        }
    }

    /// Pixels of one cell of the grid shown (blocks 16, tiles 8).
    pub(super) fn cell_px(&self) -> f32 {
        if self.in_world {
            8.0
        } else {
            BLOCK
        }
    }

    pub(super) fn brush(&self) -> &Brush {
        if self.in_world {
            &self.tiles
        } else {
            &self.blocks
        }
    }

    pub(super) fn brush_mut(&mut self) -> &mut Brush {
        if self.in_world {
            &mut self.tiles
        } else {
            &mut self.blocks
        }
    }

    fn grid_name(&self, g: Grid) -> String {
        match g {
            Grid::Level(l) => format!("{} {l:02}", tr(self.es, "level", "el nivel")),
            Grid::Map(m) => self.map_name(m).to_string(),
        }
    }

    /// What the tool paints: the clipboard while pasting, otherwise the brush.
    fn paint_stamp(&self) -> Stamp {
        match (&self.clipboard, self.tool.kind()) {
            (Some((_, s)), ToolKind::Paste) => s.clone(),
            _ => self.brush().stamp.clone(),
        }
    }

    /// Mouse on the grid: the right button picks a cell or a dragged rectangle as the brush,
    /// the left one runs the tool.
    pub(super) fn edit_grid(&mut self, cell: Option<(i32, i32)>) {
        let g = self.grid();
        if is_mouse_button_pressed(MouseButton::Right) {
            self.pick_from = cell;
        }
        if let (Some(from), true) = (self.pick_from, is_mouse_button_released(MouseButton::Right)) {
            self.pick_from = None;
            let to = cell.unwrap_or(from);
            if let Some(mut stamp) = self.doc.copy_rect(g, Area::between(from, to)) {
                if let Grid::Level(_) = g {
                    stamp.cells.iter_mut().for_each(|b| *b &= 0x7F);
                }
                let in_palette = stamp.is_single().then(|| self.palette_cell(stamp.cells[0])).map(|c| Area::between(c, c));
                *self.brush_mut() = Brush { stamp, in_palette };
            }
        }
        if self.pick_from.is_some() {
            return;
        }
        let stamp = self.paint_stamp();
        let mut changed = false;
        let mut canvas = DocCanvas { doc: &mut self.doc, grid: g, changed: &mut changed };
        self.tool.handle(&mut canvas, cell, left_button(), &stamp);
        if changed && !self.in_world {
            self.overview_stale = true;
        }
    }

    /// Where value `v` sits in the shown palette (column, row).
    pub(super) fn palette_cell(&self, v: u8) -> (i32, i32) {
        let cols = if self.in_world { 16 } else { 8 };
        ((v % cols) as i32, (v / cols) as i32)
    }

    pub(super) fn copy_selection(&mut self, cut: bool) {
        let g = self.grid();
        let Some(a) = self.tool.selection() else {
            self.status = tr(self.es, "Nothing selected (Select tool, E).", "No hay nada seleccionado (herramienta Seleccionar, E).").into();
            return;
        };
        if let Some(s) = self.doc.copy_rect(g, a) {
            self.status = format!("{} {}x{}", if cut { tr(self.es, "Cut", "Cortado") } else { tr(self.es, "Copied", "Copiado") }, s.w, s.h);
            self.clipboard = Some((g, s));
            if cut {
                self.doc.clear_rect(g, a);
                self.overview_stale = true;
            }
        }
    }

    pub(super) fn clear_selection_cells(&mut self) {
        if let Some(a) = self.tool.selection() {
            self.doc.clear_rect(self.grid(), a);
            self.overview_stale = true;
        }
    }

    pub(super) fn start_paste(&mut self) {
        match &self.clipboard {
            None => self.status = tr(self.es, "The clipboard is empty.", "El portapapeles esta vacio.").into(),
            Some((g, _)) if *g != self.grid() => {
                self.status = format!("{} {}.", tr(self.es, "The clipboard belongs to", "Lo copiado es de"), self.grid_name(*g));
            }
            Some(_) => {
                if self.tool.kind() != ToolKind::Paste {
                    self.prev_tool = self.tool.kind();
                }
                self.set_tool(ToolKind::Paste);
                self.status = tr(self.es, "Click to paste; Esc to stop.", "Clic para pegar; Esc para terminar.").into();
            }
        }
    }

    /// Esc: stop pasting, or forget the selection.
    pub(super) fn escape_grid(&mut self) {
        if self.tool.kind() == ToolKind::Paste {
            self.set_tool(self.prev_tool);
        } else {
            self.tool.clear_selection();
        }
    }

    /// The level or map shown changes: no selection or pasting carries over.
    pub(super) fn leave_grid(&mut self) {
        self.escape_grid();
        self.tool.clear_selection();
        self.pick_from = None;
    }

    /// Screen position of cell `(x, y)` of the grid shown.
    fn cell_on_screen(&self, x: i32, y: i32) -> Vec2 {
        let px = self.cell_px();
        vec2(self.panel + (x as f32 * px - self.cam.x) * self.scale(), (y as f32 * px - self.cam.y) * self.scale())
    }

    fn outline(&self, a: Area, thick: f32, c: Color) {
        let p = self.cell_on_screen(a.x0, a.y0);
        let s = self.cell_px() * self.scale();
        draw_rectangle_lines(p.x, p.y, a.w() as f32 * s, a.h() as f32 * s, thick, c);
    }

    /// Over the grid: the selection, the rectangle being dragged and what the tool would paint
    /// under the mouse.
    pub(super) fn draw_grid_overlay(&self, atlas: &Texture2D, cell: Option<(i32, i32)>) {
        if let Some(a) = self.tool.selection() {
            self.outline(a, 2.0, Color::new(1.0, 0.85, 0.1, 1.0));
        }
        let Some(c) = cell else { return };
        if let Some(from) = self.pick_from {
            self.outline(Area::between(from, c), 2.0, Color::new(0.4, 0.9, 1.0, 1.0));
            return;
        }
        if let Some(a) = self.tool.anchor() {
            self.outline(Area::between(a, c), 2.0, WHITE);
            return;
        }
        if self.tool.kind() == ToolKind::Select {
            self.outline(Area::between(c, c), 2.0, WHITE);
            return;
        }
        let stamp = self.paint_stamp();
        let px = self.cell_px();
        let s = px * self.scale();
        for dy in 0..stamp.h as i32 {
            for dx in 0..stamp.w as i32 {
                let Some(_) = self.doc.cell(self.grid(), c.0 + dx, c.1 + dy) else { continue };
                let v = stamp.at(dx, dy) & if self.in_world { 0xFF } else { 0x7F };
                let p = self.cell_on_screen(c.0 + dx, c.1 + dy);
                let src = Rect::new((v % 16) as f32 * px, (v / 16) as f32 * px, px, px);
                draw_texture_ex(atlas, p.x, p.y, Color::new(1.0, 1.0, 1.0, 0.6), DrawTextureParams { source: Some(src), dest_size: Some(vec2(s, s)), ..Default::default() });
            }
        }
        self.outline(Area { x0: c.0, y0: c.1, x1: c.0 + stamp.w as i32 - 1, y1: c.1 + stamp.h as i32 - 1 }, 2.0, WHITE);
    }
}
