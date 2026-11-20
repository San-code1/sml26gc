//! The palettes' row of what was chosen last: the last single blocks, tiles or object types
//! chosen in the level or map shown, newest first; a click chooses one again.
use super::grid_edit::Brush;
use super::palette::source;
use super::widgets::*;
use super::Editor;
use crate::domain::stamp::{Area, Stamp};
use crate::domain::tools::ToolKind;
use cart_model::original_points::OnMap;
use macroquad::prelude::*;

/// How many each row keeps.
const KEPT: usize = 8;

/// Whose row: a level's blocks or objects, or a map's tiles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Recent {
    Blocks(usize),
    Objects(usize),
    Tiles(OnMap),
}

impl Editor {
    /// The row of the palette shown.
    pub(super) fn recent_key(&self, objects: bool) -> Recent {
        if self.in_world {
            Recent::Tiles(self.world_map)
        } else if objects {
            Recent::Objects(self.cur)
        } else {
            Recent::Blocks(self.cur)
        }
    }

    /// `v` chosen in `key`'s palette: first in its row.
    pub(super) fn remember(&mut self, key: Recent, v: u8) {
        let row = match self.recent.iter_mut().find(|r| r.0 == key) {
            Some(r) => &mut r.1,
            None => {
                self.recent.push((key, Vec::new()));
                &mut self.recent.last_mut().unwrap().1
            }
        };
        row.retain(|&x| x != v);
        row.insert(0, v);
        row.truncate(KEPT);
    }

    /// The brush chosen, when it is one cell: remembered in the row of the palette shown.
    pub(super) fn remember_brush(&mut self) {
        if let Some(v) = self.brush().filter(|b| b.stamp.is_single()).map(|b| b.first()) {
            self.remember(self.recent_key(false), v);
        }
    }

    /// The row under a palette's zoom keys: its label and `KEPT` cells, the empty ones
    /// outlined, the one chosen framed. Returns where the palette continues.
    pub(super) fn recent_row(&mut self, y: f32, objects: bool) -> f32 {
        let key = self.recent_key(objects);
        let row: Vec<u8> = self.recent.iter().find(|r| r.0 == key).map(|r| r.1.clone()).unwrap_or_default();
        let label = tr(self.lang, "Recent");
        let lw = text_size(label, 14).width.ceil() + 8.0;
        let width = self.panel - 20.0;
        let cell = ((width - lw) / KEPT as f32 - 2.0).clamp(16.0, 28.0);
        put_text(label, 10.0, y + cell / 2.0 + 5.0, 14.0, col(2));
        let chosen = if objects { self.kind } else { self.brush().filter(|b| b.stamp.is_single()).map(|b| b.first()) };
        let m: Vec2 = mouse_position().into();
        let mut clicked = None;
        for i in 0..KEPT {
            let r = Rect::new(10.0 + lw + i as f32 * (cell + 2.0), y, cell, cell);
            let Some(&v) = row.get(i) else {
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, col(1));
                continue;
            };
            self.draw_recent_cell(key, v, r);
            if chosen == Some(v) {
                draw_rectangle_lines(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, 2.0, MARK_SELECTED);
            }
            if r.contains(m) {
                self.tip = Some(match self.object_label(self.look_key(v)).filter(|_| objects) {
                    Some((n, _)) => format!("#{v:02X} {n}"),
                    None => format!("#{v:02X}"),
                });
                if is_mouse_button_pressed(MouseButton::Left) {
                    clicked = Some(v);
                }
            }
        }
        if let Some(v) = clicked {
            self.choose_recent(key, v);
        }
        y + cell + 8.0
    }

    fn draw_recent_cell(&self, key: Recent, v: u8, r: Rect) {
        draw_rectangle(r.x, r.y, r.w, r.h, col(1));
        let atlas = |t: &Texture2D, px: f32| draw_texture_ex(t, r.x, r.y, WHITE, DrawTextureParams { source: Some(source(v as usize, px)), dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
        match key {
            Recent::Blocks(l) => {
                if let Some(t) = self.atlases[self.doc.base_of(l)].as_ref() {
                    atlas(t, 16.0);
                }
            }
            Recent::Tiles(_) => {
                if let Some(t) = self.map_atlas() {
                    atlas(t, 8.0);
                }
            }
            Recent::Objects(_) => match self.look(v).or_else(|| self.hidden_look(self.look_key(v))) {
                Some((t, _, _)) => {
                    let f = ((r.w - 2.0) / t.width()).min((r.h - 2.0) / t.height()).min(2.0);
                    let (w, h) = (t.width() * f, t.height() * f);
                    draw_texture_ex(t, r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
                }
                None => {
                    let t = format!("{v:02X}");
                    let d = text_size(&t, 12);
                    put_text(&t, r.x + (r.w - d.width) / 2.0, r.y + (r.h + d.offset_y) / 2.0, 12.0, col(3));
                }
            },
        }
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, col(3));
    }

    /// A recent value clicked: chosen again (a block or tile as a one-cell brush).
    fn choose_recent(&mut self, key: Recent, v: u8) {
        match key {
            Recent::Objects(_) => {
                self.kind = Some(v);
                self.sel_obj = None;
            }
            Recent::Blocks(_) | Recent::Tiles(_) => {
                if matches!(self.tool.kind(), ToolKind::Select | ToolKind::Paste) {
                    self.leave_grid();
                    self.set_tool(ToolKind::Pencil);
                }
                let c = self.palette_cell(v);
                *self.brush_mut() = Some(Brush { stamp: Stamp::single(v), in_palette: Some(Area::between(c, c)), piece: None });
            }
        }
        self.remember(key, v);
    }
}
