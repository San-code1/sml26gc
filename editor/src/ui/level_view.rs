//! The level view: blocks drawn with the game's own graphics, zoom and camera, grid, the
//! whole-level overview, painting with the chosen tool, and the side panel.
use super::widgets::*;
use super::{left_button, Editor, LevelCanvas, Mode, LEVEL_SIZE};
use crate::domain::tools::ToolKind;
use cart_model::levels::{COLS, LEVELS, ROWS};
use macroquad::prelude::*;

impl Editor {
    /// Where the whole-level overview sits (top right of the level view).
    pub(super) fn overview_rect(&self) -> Rect {
        let k = ((screen_width() - PANEL - 20.0) / COLS as f32).min(3.0);
        let (w, h) = (COLS as f32 * k, ROWS as f32 * k);
        Rect::new(screen_width() - w - 10.0, 10.0, w, h)
    }

    /// The block under the mouse (none over the panel, status bar or overview).
    pub(super) fn cell_at(&self, m: Vec2) -> Option<(i32, i32)> {
        if m.x < PANEL || m.y > screen_height() - STATUS_H {
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
        if ctrl && is_key_pressed(KeyCode::Z) {
            self.undo();
        }
        if ctrl && is_key_pressed(KeyCode::Y) {
            self.redo();
        }
        if ctrl && is_key_pressed(KeyCode::S) {
            self.save();
        }
        if is_key_pressed(KeyCode::F5) {
            self.play();
        }
        self.tool_keys(ctrl);
        let (space, view, max) = self.navigate(m, ctrl, LEVEL_SIZE);

        if !ctrl && is_key_pressed(KeyCode::M) {
            self.show_overview = !self.show_overview;
        }
        // Click or drag on the overview: centre the view there.
        let ov = self.overview_rect();
        if self.show_overview && ov.contains(m) && is_mouse_button_down(MouseButton::Left) {
            let k = ov.w / COLS as f32;
            let target = vec2((m.x - ov.x) / k, (m.y - ov.y) / k) * BLOCK;
            self.cam = (target - view * 0.5).clamp(Vec2::ZERO, max);
            return;
        }
        if !ctrl && (is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::O)) {
            self.mode = if self.mode == Mode::Blocks { Mode::Objects } else { Mode::Blocks };
            self.sel_obj = None;
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
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some((c, r)) = cell {
                self.block = self.doc.block(self.cur, c, r) & 0x7F;
            }
        }
        let mut canvas = LevelCanvas { doc: &mut self.doc, level: self.cur, block: self.block, changed: &mut self.overview_stale };
        self.tool.handle(&mut canvas, cell, left_button());
    }

    pub(super) fn draw_level(&mut self) {
        clear_background(col(3));
        let s = self.scale() * BLOCK;
        let (cw, ch) = (screen_width() - PANEL, screen_height() - STATUS_H);
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
                    let x = PANEL + (c as f32 * BLOCK - self.cam.x) * self.scale();
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
                    let x = PANEL + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    draw_line(x, 0.0, x, ch, 1.0, gc);
                }
                for r in r0..=r0 + nrow {
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_line(PANEL, y, screen_width(), y, 1.0, gc);
                }
            }
            // where the level ends
            let end_x = PANEL + (COLS as f32 * BLOCK - self.cam.x) * self.scale();
            let end_y = (ROWS as f32 * BLOCK - self.cam.y) * self.scale();
            draw_rectangle_lines(PANEL - self.cam.x * self.scale(), -self.cam.y * self.scale(), end_x - (PANEL - self.cam.x * self.scale()), end_y + self.cam.y * self.scale(), 3.0, col(3));
            // hover / rectangle preview
            let m: Vec2 = mouse_position().into();
            if let (Mode::Blocks, Some((c, r))) = (self.mode, self.cell_at(m)) {
                let anchor = self.tool.anchor();
                let (a, b) = match anchor {
                    Some((c0, r0)) => ((c0.min(c), r0.min(r)), (c0.max(c), r0.max(r))),
                    None => ((c, r), (c, r)),
                };
                let x = PANEL + (a.0 as f32 * BLOCK - self.cam.x) * self.scale();
                let y = (a.1 as f32 * BLOCK - self.cam.y) * self.scale();
                let w = (b.0 - a.0 + 1) as f32 * s;
                let h = (b.1 - a.1 + 1) as f32 * s;
                if anchor.is_none() {
                    draw_texture_ex(
                        atlas,
                        x,
                        y,
                        Color::new(1.0, 1.0, 1.0, 0.6),
                        DrawTextureParams {
                            source: Some(Rect::new((self.block % 16) as f32 * 16.0, (self.block / 16) as f32 * 16.0, 16.0, 16.0)),
                            dest_size: Some(vec2(s, s)),
                            ..Default::default()
                        },
                    );
                }
                draw_rectangle_lines(x, y, w, h, 2.0, Color::new(1.0, 1.0, 1.0, 0.9));
                let id = self.doc.block(self.cur, c, r);
                draw_text(&format!("{c},{r}  #{id:02X}"), PANEL + 8.0, ch - 8.0, 20.0, WHITE);
            }
        } else {
            draw_text(tr(self.es, "Loading the level from the game...", "Cargando el nivel desde el juego..."), PANEL + 30.0, 60.0, 26.0, col(0));
        }
        self.draw_objects();
        if self.mode == Mode::Objects {
            self.object_tip(ch);
        }
        self.draw_overview(cw, ch);
        let (act, pick) = self.level_panel();
        self.level_status_bar(cw, ch);

        match act {
            1 => self.undo(),
            2 => self.redo(),
            3 => self.restore_original(),
            4 => {
                self.save();
            }
            5 => self.play(),
            9 => self.enter_world(true),
            10 => self.new_level(),
            11 => self.delete_level(),
            _ => {}
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
    fn level_panel(&mut self) -> (u32, Option<usize>) {
        draw_rectangle(0.0, 0.0, PANEL, screen_height(), col(0));
        draw_rectangle(PANEL - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(tr(self.es, "LEVEL EDITOR", "EDITOR DE NIVELES"), 10.0, 22.0, 24.0, col(3));
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
        let bw = 92.0;
        let mut act = 0;
        // new levels: a copy of this one, played in place of the level it is based on
        if button(Rect::new(10.0, y, 136.0, 24.0), tr(self.es, "New level (copy)", "Nivel nuevo (copia)"), false, 15.0) {
            act = 10;
        }
        if self.cur >= LEVELS {
            if button(Rect::new(150.0, y, 136.0, 24.0), tr(self.es, "Delete level", "Borrar nivel"), false, 15.0) {
                act = 11;
            }
            y += 28.0;
            let t = format!("{} {:02}", tr(self.es, "plays as level", "se juega como el nivel"), self.doc.base_of(self.cur));
            draw_text(&t, 14.0, y + 14.0, 16.0, col(2));
            y += 4.0;
        }
        y += 28.0;
        if button(Rect::new(10.0, y, 276.0, 24.0), tr(self.es, "World map editor", "Editor del mapa de mundos"), false, 16.0) {
            act = 9;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 136.0, 26.0), tr(self.es, "Blocks (Tab)", "Bloques (Tab)"), self.mode == Mode::Blocks, 16.0) {
            self.mode = Mode::Blocks;
            self.sel_obj = None;
        }
        if button(Rect::new(150.0, y, 136.0, 26.0), tr(self.es, "Objects (Tab)", "Objetos (Tab)"), self.mode == Mode::Objects, 16.0) {
            self.mode = Mode::Objects;
        }
        y += 30.0;
        if self.mode == Mode::Blocks {
            if button(Rect::new(10.0, y, bw, 26.0), tr(self.es, "Pencil (B)", "Lapiz (B)"), self.tool.kind() == ToolKind::Pencil, 16.0) {
                self.set_tool(ToolKind::Pencil);
            }
            if button(Rect::new(10.0 + bw + 4.0, y, bw, 26.0), tr(self.es, "Box (R)", "Caja (R)"), self.tool.kind() == ToolKind::Box, 16.0) {
                self.set_tool(ToolKind::Box);
            }
        } else {
            let n = self.doc.levels[self.cur].content.objs.len();
            draw_text(&format!("{} {n}", tr(self.es, "objects:", "objetos:")), 14.0, y + 18.0, 18.0, col(3));
        }
        if button(Rect::new(10.0 + 2.0 * (bw + 4.0), y, bw - 8.0, 26.0), tr(self.es, "Grid (G)", "Rejilla (G)"), self.grid, 16.0) {
            self.grid = !self.grid;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, bw, 26.0), tr(self.es, "Undo", "Deshacer"), false, 16.0) {
            act = 1;
        }
        if button(Rect::new(10.0 + bw + 4.0, y, bw, 26.0), tr(self.es, "Redo", "Rehacer"), false, 16.0) {
            act = 2;
        }
        if button(Rect::new(10.0 + 2.0 * (bw + 4.0), y, bw - 8.0, 26.0), tr(self.es, "Original", "Original"), false, 16.0) {
            act = 3;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 136.0, 28.0), tr(self.es, "Save (Ctrl+S)", "Guardar (Ctrl+S)"), false, 17.0) {
            act = 4;
        }
        if button(Rect::new(150.0, y, 136.0, 28.0), tr(self.es, "Play (F5)", "Jugar (F5)"), false, 17.0) {
            act = 5;
        }
        y += 36.0;
        if self.mode == Mode::Objects {
            self.object_palette(y);
        } else {
            self.block_palette(y);
        }
        (act, pick)
    }
}
