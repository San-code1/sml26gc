//! The side panel's palettes: the level's 128 blocks, the 128 object types and the shown map's
//! 256 tiles. Each is zoomed with its own setting and keeps its layout; when it is bigger than
//! the panel it scrolls (wheel, right-drag) and shows scroll bars. Dragging over the blocks or
//! tiles selects a rectangle of them.
use super::grid_edit::Brush;
use super::shortcuts::{self, Action};
use super::widgets::*;
use super::testing_view::warn_col;
use super::Editor;
use cart_model::{checkpoint, level_graphics};
use crate::domain::stamp::{Area, Stamp};
use crate::domain::tools::ToolKind;
use macroquad::prelude::*;

/// Draw only inside `r` (or everywhere with None).
pub(super) fn clip(r: Option<Rect>) {
    let d = screen_dpi_scale();
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.scissor(r.map(|r| ((r.x * d) as i32, (r.y * d) as i32, (r.w * d) as i32, (r.h * d) as i32)));
}

/// Panel pixels beside a palette: its left margin, and the scroll bar and border on the right.
const MARGINS: f32 = 24.0;

/// A palette's layout: `count` cells, `cols` per row, of `px` source pixels, `gap` pixels apart.
#[derive(Clone, Copy)]
struct Layout {
    count: usize,
    cols: usize,
    px: f32,
    gap: f32,
}

impl Layout {
    const BLOCKS: Layout = Layout { count: 128, cols: 8, px: 16.0, gap: 2.0 };
    const TILES: Layout = Layout { count: 256, cols: 16, px: 8.0, gap: 1.0 };

    fn cell(&self, zoom: u32) -> f32 {
        self.px * zoom as f32 + self.gap
    }

    fn width(&self, zoom: u32) -> f32 {
        self.cols as f32 * self.cell(zoom)
    }
}

/// How a palette is scrolled: its offset and a right-button drag (mouse and offset when it
/// started).
#[derive(Clone, Copy, Default)]
pub struct PaletteView {
    scroll: Vec2,
    drag: Option<(Vec2, Vec2)>,
}

/// The cells drawn by `draw` inside `area`, scrolled by the wheel (Shift: sideways) or a
/// right-drag, with scroll bars when cut; `outline` frames a rectangle of cells. Returns the
/// cell under the mouse (column, row), none while dragging.
fn cell_grid(area: Rect, l: Layout, cell: f32, view: &mut PaletteView, outline: Option<Area>, mut draw: impl FnMut(usize, Rect)) -> Option<(i32, i32)> {
    let size = vec2(l.cols as f32 * cell, l.count.div_ceil(l.cols) as f32 * cell);
    let m: Vec2 = mouse_position().into();
    let over = area.contains(m);
    let wheel = mouse_wheel().1;
    if over && wheel != 0.0 {
        let step = -wheel.signum() * cell * 2.0;
        if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
            view.scroll.x += step;
        } else {
            view.scroll.y += step;
        }
    }
    if over && is_mouse_button_pressed(MouseButton::Right) {
        view.drag = Some((m, view.scroll));
    }
    if let Some((m0, s0)) = view.drag {
        if is_mouse_button_down(MouseButton::Right) {
            view.scroll = s0 - (m - m0);
        } else {
            view.drag = None;
        }
    }
    let max = (size - area.size()).max(Vec2::ZERO);
    view.scroll = view.scroll.clamp(Vec2::ZERO, max);
    let scroll = view.scroll;
    let at = |c: i32, r: i32| vec2(area.x + c as f32 * cell - scroll.x, area.y + r as f32 * cell - scroll.y);
    clip(Some(area));
    for i in 0..l.count {
        let p = at((i % l.cols) as i32, (i / l.cols) as i32);
        let r = Rect::new(p.x, p.y, cell - l.gap, cell - l.gap);
        if r.overlaps(&area) {
            draw(i, r);
        }
    }
    if let Some(a) = outline {
        let p = at(a.x0, a.y0);
        draw_rectangle_lines(p.x - 1.0, p.y - 1.0, a.w() as f32 * cell, a.h() as f32 * cell, 3.0, col(3));
    }
    clip(None);
    let bar = Color::new(0.12, 0.12, 0.12, 0.8);
    if max.x > 0.0 {
        let w = area.w * area.w / size.x;
        draw_rectangle(area.x + (area.w - w) * scroll.x / max.x, area.y + area.h + 1.0, w, 4.0, bar);
    }
    if max.y > 0.0 {
        let h = area.h * area.h / size.y;
        draw_rectangle(area.x + area.w + 2.0, area.y + (area.h - h) * scroll.y / max.y, 4.0, h, bar);
    }
    if !over || view.drag.is_some() {
        return None;
    }
    let c = ((m.x - area.x + scroll.x) / cell).floor() as i32;
    let r = ((m.y - area.y + scroll.y) / cell).floor() as i32;
    (c >= 0 && (c as usize) < l.cols && r >= 0 && (r as usize * l.cols + c as usize) < l.count).then_some((c, r))
}

/// Cell `i` of a 16-per-row source picture of `px`-pixel cells.
pub(super) fn source(i: usize, px: f32) -> Rect {
    Rect::new((i % 16) as f32 * px, (i / 16) as f32 * px, px, px)
}

impl Editor {
    /// The side panel's width for the palette shown: all its columns, but never narrower
    /// than the buttons nor wider than half the window.
    pub(super) fn fitted_panel(&self) -> f32 {
        let w = if self.in_world { Layout::TILES.width(self.prefs.map_palette_zoom) } else { Layout::BLOCKS.width(self.prefs.level_palette_zoom) };
        (w + MARGINS).clamp(PANEL_MIN, (screen_width() / 2.0).max(PANEL_MIN))
    }

    /// The palette's title, then its zoom row; returns where the palette starts.
    fn palette_title(&mut self, y: f32, title: &str) -> f32 {
        draw_text(title, 10.0, y + 14.0, 18.0, col(3));
        let y = y + 20.0;
        let keys = format!("Zoom  Ctrl {}/{}", shortcuts::keys(Action::ZoomOut).trim_start_matches("Ctrl+"), shortcuts::keys(Action::ZoomIn).trim_start_matches("Ctrl+"));
        draw_text(&keys, 10.0, y + 15.0, 15.0, col(2));
        let right = self.panel - 10.0;
        if button(Rect::new(right - 54.0, y, 26.0, 20.0), "-", false, 18.0) {
            self.run(Action::ZoomOut);
        }
        if button(Rect::new(right - 26.0, y, 26.0, 20.0), "+", false, 18.0) {
            self.run(Action::ZoomIn);
        }
        y + 26.0
    }

    /// Where a palette goes: from `y` down to above its tip line, leaving room for the bars.
    fn palette_area(&self, y: f32) -> Rect {
        Rect::new(10.0, y, self.panel - MARGINS, (screen_height() - y - 30.0).max(0.0))
    }

    fn tip(text: &str, area: Rect) {
        draw_text(text, 10.0, area.y + area.h + 22.0, 15.0, col(2));
    }

    fn brush_title(&self, what: &str) -> String {
        let b = self.brush();
        let size = if b.stamp.is_single() { String::new() } else { format!("  {}x{}", b.stamp.w, b.stamp.h) };
        format!("{what} #{:02X}{size}", b.first())
    }

    /// Mouse on a block or tile palette: a click or a drag chooses a rectangle of cells.
    fn palette_select(&mut self, hover: Option<(i32, i32)>, l: Layout) {
        if let (true, Some(c)) = (is_mouse_button_pressed(MouseButton::Left), hover) {
            self.palette_drag = Some(c);
            if matches!(self.tool.kind(), ToolKind::Select | ToolKind::Paste) {
                self.leave_grid();
                self.set_tool(ToolKind::Pencil);
            }
        }
        let Some(from) = self.palette_drag else { return };
        if let Some(to) = hover {
            let a = Area::between(from, to);
            let cells = a.cells().map(|(x, y)| (y as usize * l.cols + x as usize).min(l.count - 1) as u8).collect();
            *self.brush_mut() = Brush { stamp: Stamp { w: a.w(), h: a.h(), cells }, in_palette: Some(a) };
        }
        if !is_mouse_button_down(MouseButton::Left) {
            self.palette_drag = None;
        }
    }

    /// Object types 00-7F; the ones this level uses are dark.
    pub(super) fn object_palette(&mut self, y: f32) {
        let title = match self.kind {
            Some(k) => format!("{} #{k:02X}", tr(self.es, "OBJECTS", "OBJETOS")),
            None => tr(self.es, "OBJECTS", "OBJETOS").to_string(),
        };
        let y = self.palette_title(y, &title);
        let all_label = tr(self.es, "Show all", "Mostrar todos");
        if button(Rect::new(self.panel - 10.0 - 130.0, y - 46.0, 130.0, 18.0), all_label, self.show_all_objects, 14.0) {
            self.show_all_objects = !self.show_all_objects;
        }
        let area = self.palette_area(y);
        let zoom = self.prefs.level_palette_zoom;
        let cell = Layout::BLOCKS.cell(zoom);
        let used: Vec<u8> = self.doc.levels[self.cur].content.objs.iter().map(|o| o.kind).collect();
        let offered = self.offered_types(self.doc.base_of(self.cur));
        let shown: Vec<u8> = if self.show_all_objects { (0..128).collect() } else { offered.clone() };
        let layout = Layout { count: shown.len(), ..Layout::BLOCKS };
        let mut view = self.level_palette;
        let hover = cell_grid(area, layout, cell, &mut view, None, |i, r| {
            let k = shown[i];
            let (bg, fg) = if Some(k) == self.kind { (col(3), col(0)) } else if used.contains(&k) { (col(2), col(0)) } else { (col(1), col(3)) };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            if !offered.contains(&k) {
                draw_rectangle_lines(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, warn_col());
            }
            let fs = (6.0 + 5.0 * zoom as f32).min(18.0);
            if let Some((tex, _, _)) = self.look(k) {
                let f = ((r.w - 4.0) / tex.width()).min((r.h - 4.0) / tex.height()).min(zoom as f32);
                let (tw, th) = (tex.width() * f, tex.height() * f);
                draw_texture_ex(tex, r.x + (r.w - tw) / 2.0, r.y + (r.h - th) / 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(tw, th)), ..Default::default() });
                draw_text(&format!("{k:02X}"), r.x + 1.0, r.y + fs * 0.7, fs * 0.7, fg);
            } else {
                let t = format!("{k:02X}");
                let dim = measure_text(&t, None, fs as u16, 1.0);
                let known = self.looks.contains_key(&(self.doc.base_of(self.cur), k));
                draw_text(&t, r.x + (r.w - dim.width) / 2.0, r.y + (r.h + dim.offset_y) / 2.0, fs, if known { fg } else { Color::new(fg.r, fg.g, fg.b, 0.5) });
            }
        });
        self.level_palette = view;
        if let (true, Some(&k)) = (is_mouse_button_pressed(MouseButton::Left), hover.and_then(|(c, r)| shown.get((r * 8 + c) as usize))) {
            self.choose_kind(k);
        }
        let tip = if self.show_all_objects {
            tr(self.es, "Framed: may look broken in this level.", "Con marco: pueden verse rotos en este nivel.").to_string()
        } else {
            format!("{} {}", tr(self.es, "Dark: used here. Delete chosen:", "Oscuros: usados aqui. Borrar elegido:"), shortcuts::keys(Action::DeleteObject))
        };
        Self::tip(&tip, area);
    }

    /// The object types game level `base` draws right (`level_graphics`): the bells, those the
    /// game's levels with its sprites and catalogue use, and, once its picture there is
    /// captured, a type a game level of its catalogue uses that loads the same graphics into
    /// the sprite sets the picture uses or draws it with the same pixels.
    pub(super) fn offered_types(&mut self, base: usize) -> Vec<u8> {
        if !self.offered.contains_key(&base) {
            let users = (0..128u8).map(|k| (k, level_graphics::levels_using(&self.doc.original, base, k))).filter(|(_, l)| !l.is_empty()).collect();
            self.offered.insert(base, users);
        }
        let group = level_graphics::same_objects(&self.doc.original, base);
        let users = &self.offered[&base];
        let original = &self.doc.original;
        (0..128u8)
            .filter(|&k| {
                if k == cart_model::level_goals::BELL || k == checkpoint::BELL {
                    return true;
                }
                let Some((_, levels)) = users.iter().find(|(u, _)| *u == k) else { return false };
                let Some(here) = self.captured.get(&(base, k)) else { return levels.iter().any(|l| group.contains(l)) };
                levels.iter().any(|&l| group.contains(&l) || level_graphics::same_sets(original, base, l, here.sets) || self.captured.get(&(l, k)).is_some_and(|there| (there.w, there.h, &there.px) == (here.w, here.h, &here.px)))
            })
            .collect()
    }

    /// The level's blocks as the game draws them, 8 per row.
    pub(super) fn block_palette(&mut self, y: f32) {
        let title = self.brush_title(tr(self.es, "BLOCKS", "BLOQUES"));
        let y = self.palette_title(y, &title);
        let area = self.palette_area(y);
        let Some(atlas) = self.atlases[self.doc.base_of(self.cur)].clone() else { return };
        let cell = Layout::BLOCKS.cell(self.prefs.level_palette_zoom);
        let mut view = self.level_palette;
        let hover = cell_grid(area, Layout::BLOCKS, cell, &mut view, self.blocks.in_palette, |i, r| {
            draw_texture_ex(&atlas, r.x, r.y, WHITE, DrawTextureParams { source: Some(source(i, 16.0)), dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
        });
        self.level_palette = view;
        self.palette_select(hover, Layout::BLOCKS);
        Self::tip(tr(self.es, "Drag: several. Right drag: move it", "Arrastra: varios. Boton derecho: moverla"), area);
    }

    /// The shown map's 256 tiles, 16 per row.
    pub(super) fn tile_palette(&mut self, y: f32) {
        let title = self.brush_title("TILES");
        let y = self.palette_title(y, &title);
        let area = self.palette_area(y);
        let Some(atlas) = self.map_atlas().cloned() else { return };
        let cell = Layout::TILES.cell(self.prefs.map_palette_zoom);
        let mut view = self.map_palette;
        let hover = cell_grid(area, Layout::TILES, cell, &mut view, self.tiles.in_palette, |i, r| {
            draw_texture_ex(&atlas, r.x, r.y, WHITE, DrawTextureParams { source: Some(source(i, 8.0)), dest_size: Some(vec2(r.w + 1.0, r.h + 1.0)), ..Default::default() });
        });
        self.map_palette = view;
        self.palette_select(hover, Layout::TILES);
        Self::tip(tr(self.es, "Drag: several. Right drag: move it", "Arrastra: varios. Boton derecho: moverla"), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_at_the_default_zoom_fit_the_narrowest_panel() {
        assert!(Layout::TILES.width(2) + MARGINS <= PANEL_MIN);
        assert!(Layout::BLOCKS.width(2) + MARGINS <= PANEL_MIN);
        assert_eq!(Layout::TILES.width(4), 16.0 * 33.0);
    }
}
