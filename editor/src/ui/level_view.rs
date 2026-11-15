//! The level view: blocks drawn with the game's own graphics, zoom and camera, grid, the
//! whole-level overview, painting with the chosen tool, and the side panel.
use super::shortcuts::{with_keys, Action};
use super::widgets::*;
use super::{Editor, Mode, LEVEL_SIZE};
use crate::domain::tools::ToolKind;
use cart_model::levels::{COLS, LEVELS, ROWS};
use macroquad::prelude::*;

impl Editor {
    /// Where the whole-level overview sits (top right of the level view).
    pub(super) fn overview_rect(&self) -> Rect {
        let k = ((screen_width() - self.panel - 20.0) / COLS as f32).min(3.0);
        let (w, h) = (COLS as f32 * k, ROWS as f32 * k);
        Rect::new(screen_width() - w - 10.0, 10.0, w, h)
    }

    /// The block under the mouse (none over the panel, status bar or overview).
    pub(super) fn cell_at(&self, m: Vec2) -> Option<(i32, i32)> {
        if m.x < self.panel || m.y > screen_height() - STATUS_H {
            return None;
        }
        if self.show_overview && self.overview_rect().contains(m) {
            return None;
        }
        let w = self.world_pos(m);
        let (c, r) = ((w.x / BLOCK).floor() as i32, (w.y / BLOCK).floor() as i32);
        if c >= 0 && r >= 0 && (c as usize) < COLS && (r as usize) < ROWS {
            Some((c, r))
        } else {
            None
        }
    }

    pub(super) fn update_level(&mut self) {
        let m: Vec2 = mouse_position().into();
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let (space, view, max) = self.navigate(m, ctrl, LEVEL_SIZE);
        // Click or drag on the overview: centre the view there.
        let ov = self.overview_rect();
        if self.show_overview && ov.contains(m) && is_mouse_button_down(MouseButton::Left) {
            let k = ov.w / COLS as f32;
            let target = vec2((m.x - ov.x) / k, (m.y - ov.y) / k) * BLOCK;
            self.cam = (target - view * 0.5).clamp(Vec2::ZERO, max);
            return;
        }
        if self.pan.is_some() || space {
            return;
        }
        if self.mode == Mode::Objects {
            self.pump_looks();
            self.update_objects(m);
            return;
        }
        let cell = self.cell_at(m);
        self.edit_grid(cell);
    }

    pub(super) fn draw_level(&mut self) {
        clear_background(col(3));
        let s = self.scale() * BLOCK;
        let (cw, ch) = (screen_width() - self.panel, screen_height() - STATUS_H);
        // --- the level
        if let Some(atlas) = &self.atlases[self.doc.base_of(self.cur)] {
            let c0 = (self.cam.x / BLOCK).floor() as i32;
            let r0 = (self.cam.y / BLOCK).floor() as i32;
            let ncol = (cw / s).ceil() as i32 + 1;
            let nrow = (ch / s).ceil() as i32 + 1;
            let map = &self.doc.levels[self.cur].content.map;
            for r in r0..(r0 + nrow).min(ROWS as i32) {
                for c in c0..(c0 + ncol).min(COLS as i32) {
                    let id = map[r as usize * COLS + c as usize] & 0x7F;
                    let x = self.panel + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_texture_ex(
                        atlas,
                        x.floor(),
                        y.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new((id % 16) as f32 * 16.0, (id / 16) as f32 * 16.0, 16.0, 16.0)),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            if self.grid && s >= 16.0 {
                let gc = Color::new(0.0, 0.0, 0.0, 0.15);
                for c in c0..=c0 + ncol {
                    let x = self.panel + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    draw_line(x, 0.0, x, ch, 1.0, gc);
                }
                for r in r0..=r0 + nrow {
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_line(self.panel, y, screen_width(), y, 1.0, gc);
                }
            }
            // where the level ends
            let end_x = self.panel + (COLS as f32 * BLOCK - self.cam.x) * self.scale();
            let end_y = (ROWS as f32 * BLOCK - self.cam.y) * self.scale();
            draw_rectangle_lines(self.panel - self.cam.x * self.scale(), -self.cam.y * self.scale(), end_x - (self.panel - self.cam.x * self.scale()), end_y + self.cam.y * self.scale(), 3.0, col(3));
            let m: Vec2 = mouse_position().into();
            if self.mode == Mode::Blocks {
                let cell = self.cell_at(m);
                self.draw_grid_overlay(atlas, cell);
                if let Some((c, r)) = cell {
                    let id = self.doc.block(self.cur, c, r);
                    draw_text(&format!("{c},{r}  #{id:02X}"), self.panel + 8.0, ch - 8.0, 20.0, WHITE);
                }
            }
        } else {
            draw_text(tr(self.es, "Loading the level from the game...", "Cargando el nivel desde el juego..."), self.panel + 30.0, 60.0, 26.0, col(0));
        }
        self.finish_level_frame(cw, ch);
    }

    /// Objects, overview, panel and status bar, over the level.
    fn finish_level_frame(&mut self, cw: f32, ch: f32) {
        self.draw_objects();
        if self.mode == Mode::Objects {
            self.object_tip(ch);
        }
        self.draw_overview(cw, ch);
        let (act, pick) = self.level_panel();
        self.level_status_bar(cw, ch);
        if let Some(a) = act {
            self.run(a);
        }
        if let Some(l) = pick {
            self.select_level(l);
        }
    }

    /// The whole level, one pixel per block (M), with the part on screen outlined.
    fn draw_overview(&mut self, cw: f32, ch: f32) {
        if !self.show_overview {
            return;
        }
        if self.overview_stale || self.overview.is_none() {
            if let Some(art) = self.art.level(self.doc.base_of(self.cur)) {
                self.overview = Some(make_overview(art, &self.doc.levels[self.cur].content.map));
                self.overview_stale = false;
            }
        }
        if let Some(t) = &self.overview {
            let r = self.overview_rect();
            let k = r.w / COLS as f32;
            draw_rectangle(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, col(3));
            draw_texture_ex(t, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
            let v = vec2(cw, ch) / self.scale() / BLOCK * k;
            let p = self.cam / BLOCK * k;
            draw_rectangle_lines(r.x + p.x, r.y + p.y, v.x.min(r.w), v.y.min(r.h), 2.0, Color::from_rgba(255, 255, 255, 230));
            draw_text(tr(self.es, "MAP (M)", "MAPA (M)"), r.x + 4.0, r.y + r.h + 16.0, 16.0, col(0));
        }
    }

    /// Level buttons, mode, tools, undo, save, play and the palette. Returns the action
    /// chosen (done after drawing) and the level picked.
    fn level_panel(&mut self) -> (Option<Action>, Option<usize>) {
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(tr(self.es, "LEVEL EDITOR", "EDITOR DE NIVELES"), 10.0, 22.0, 22.0, col(3));
        let keys = format!("{} / {}", super::shortcuts::keys(Action::PrevLevel), super::shortcuts::keys(Action::NextLevel));
        let d = measure_text(&keys, None, 14, 1.0);
        draw_text(&keys, self.panel - 12.0 - d.width, 22.0, 14.0, col(2));
        let mut y = 34.0;
        let mut pick = None;
        let count = self.doc.levels.len();
        for l in 0..count {
            let (cx, cy) = (10.0 + (l % 8) as f32 * 35.0, y + (l / 8) as f32 * 25.0);
            let label = format!("{:02}{}", l, if self.doc.levels[l].dirty() { "*" } else { "" });
            if button(Rect::new(cx, cy, 33.0, 23.0), &label, l == self.cur, 16.0) {
                pick = Some(l);
            }
        }
        y += count.div_ceil(8) as f32 * 25.0 + 4.0;
        let es = self.es;
        let mut act = None;
        let mut b = |r: Rect, name: &str, a: Action, on: bool| {
            if button(r, &with_keys(name, a), on, 16.0) {
                act = Some(a);
            }
        };
        let (w2, w3) = (136.0, 89.0);
        let col2 = |i: usize| 10.0 + i as f32 * (w2 + 4.0);
        let col3 = |i: usize| 10.0 + i as f32 * (w3 + 4.0);
        // new levels: a copy of this one, played in place of the level it is based on
        b(Rect::new(col2(0), y, w2, 24.0), tr(es, "New level", "Nivel nuevo"), Action::NewLevel, false);
        if self.cur >= LEVELS {
            b(Rect::new(col2(1), y, w2, 24.0), tr(es, "Delete level", "Borrar nivel"), Action::DeleteLevel, false);
            y += 28.0;
            let t = format!("{} {:02}", tr(es, "plays as level", "se juega como el nivel"), self.doc.base_of(self.cur));
            draw_text(&t, 14.0, y + 14.0, 16.0, col(2));
            y += 4.0;
        }
        y += 28.0;
        b(Rect::new(10.0, y, 276.0, 24.0), tr(es, "World map editor", "Editor del mapa de mundos"), Action::WorldEditor, false);
        y += 30.0;
        let blocks = self.mode == Mode::Blocks;
        if button(Rect::new(col2(0), y, w2, 26.0), &with_keys(tr(es, "Blocks", "Bloques"), Action::BlocksObjects), blocks, 16.0) {
            self.set_mode(Mode::Blocks);
        }
        if button(Rect::new(col2(1), y, w2, 26.0), &with_keys(tr(es, "Objects", "Objetos"), Action::BlocksObjects), !blocks, 16.0) {
            self.set_mode(Mode::Objects);
        }
        y += 30.0;
        let tool = self.tool.kind();
        if self.mode == Mode::Blocks {
            b(Rect::new(col3(0), y, w3, 26.0), tr(es, "Pencil", "Lapiz"), Action::Pencil, tool == ToolKind::Pencil);
            b(Rect::new(col3(1), y, w3, 26.0), tr(es, "Box", "Caja"), Action::BoxFill, tool == ToolKind::Box);
            b(Rect::new(col3(2), y, w3, 26.0), tr(es, "Select", "Seleccion"), Action::Select, tool == ToolKind::Select);
        } else {
            let n = self.doc.levels[self.cur].content.objs.len();
            draw_text(&format!("{} {n}", tr(es, "objects:", "objetos:")), 14.0, y + 18.0, 18.0, col(3));
        }
        y += 30.0;
        b(Rect::new(col2(0), y, w2, 26.0), tr(es, "Undo", "Deshacer"), Action::Undo, false);
        b(Rect::new(col2(1), y, w2, 26.0), tr(es, "Redo", "Rehacer"), Action::Redo, false);
        y += 30.0;
        b(Rect::new(col3(0), y, w3, 26.0), tr(es, "Grid", "Rejilla"), Action::Grid, self.grid);
        b(Rect::new(col3(1), y, 276.0 - w3 - 4.0, 26.0), "Original", Action::Original, false);
        y += 30.0;
        b(Rect::new(col2(0), y, w2, 28.0), tr(es, "Save", "Guardar"), Action::Save, false);
        b(Rect::new(col2(1), y, w2, 28.0), tr(es, "Play", "Jugar"), Action::Play, false);
        y += 34.0;
        if self.mode == Mode::Objects {
            self.object_palette(y);
        } else {
            self.block_palette(y);
        }
        (act, pick)
    }
}
