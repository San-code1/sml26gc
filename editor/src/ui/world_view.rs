//! The world map editor: the map picture painted tile by tile (Picture tab) and the level
//! each of the game's own points opens (Orig. points tab). The third tab, the editor-made
//! map, is in `custom_view`.
use super::widgets::*;
use super::{left_button, Editor, WorldCanvas, TAB_CUSTOM, TAB_PICTURE, TAB_POINTS};
use crate::domain::tools::ToolKind;
use cart_model::world;
use macroquad::prelude::*;

const WORLD_SIZE: Vec2 = Vec2::new(world::W as f32 * 8.0, world::H as f32 * 8.0);

impl Editor {
    /// Tile under the mouse on the world map.
    pub(super) fn world_cell(&self, m: Vec2) -> Option<(usize, usize)> {
        if m.x < PANEL || m.y > screen_height() - STATUS_H {
            return None;
        }
        let w = self.world_pos(m);
        let (x, y) = ((w.x / 8.0).floor() as i32, (w.y / 8.0).floor() as i32);
        if x >= 0 && y >= 0 && (x as usize) < world::W && (y as usize) < world::H {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    pub(super) fn update_world(&mut self) {
        let m: Vec2 = mouse_position().into();
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::Z) {
            self.doc.world_undo();
        }
        if ctrl && is_key_pressed(KeyCode::Y) {
            self.doc.world_redo();
        }
        if ctrl && is_key_pressed(KeyCode::S) {
            self.save();
        }
        self.tool_keys(ctrl);
        let (space, _, _) = self.navigate(m, ctrl, WORLD_SIZE);
        if self.pan.is_some() || space {
            return;
        }
        if self.world_tab == TAB_CUSTOM {
            self.update_custom(m);
            return;
        }
        let cell = self.world_cell(m);
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some((x, y)) = cell {
                self.tile = self.doc.tile(x, y);
            }
        }
        let mut canvas = WorldCanvas { doc: &mut self.doc, tile: self.tile };
        self.tool.handle(&mut canvas, cell.map(|(x, y)| (x as i32, y as i32)), left_button());
    }

    pub(super) fn draw_world(&mut self) {
        clear_background(col(3));
        let s = self.scale() * 8.0;
        let (cw, ch) = (screen_width() - PANEL, screen_height() - STATUS_H);
        if let Some(atlas) = &self.world_atlas {
            let x0 = (self.cam.x / 8.0).floor() as i32;
            let y0 = (self.cam.y / 8.0).floor() as i32;
            let nx = (cw / s).ceil() as i32 + 1;
            let ny = (ch / s).ceil() as i32 + 1;
            for y in y0..(y0 + ny).min(world::H as i32) {
                for x in x0..(x0 + nx).min(world::W as i32) {
                    let t = self.doc.tile(x as usize, y as usize);
                    let px = PANEL + (x as f32 * 8.0 - self.cam.x) * self.scale();
                    let py = (y as f32 * 8.0 - self.cam.y) * self.scale();
                    draw_texture_ex(
                        atlas,
                        px.floor(),
                        py.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0)),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            if self.grid && s >= 12.0 {
                let gc = Color::new(0.0, 0.0, 0.0, 0.12);
                for x in x0..=x0 + nx {
                    let px = PANEL + (x as f32 * 8.0 - self.cam.x) * self.scale();
                    draw_line(px, 0.0, px, ch, 1.0, gc);
                }
                for y in y0..=y0 + ny {
                    let py = (y as f32 * 8.0 - self.cam.y) * self.scale();
                    draw_line(PANEL, py, screen_width(), py, 1.0, gc);
                }
            }
            let m: Vec2 = mouse_position().into();
            if self.world_tab == TAB_CUSTOM {
                self.draw_custom();
            } else if let Some((x, y)) = self.world_cell(m) {
                let px = PANEL + (x as f32 * 8.0 - self.cam.x) * self.scale();
                let py = (y as f32 * 8.0 - self.cam.y) * self.scale();
                draw_texture_ex(
                    atlas,
                    px,
                    py,
                    Color::new(1.0, 1.0, 1.0, 0.6),
                    DrawTextureParams {
                        source: Some(Rect::new((self.tile % 16) as f32 * 8.0, (self.tile / 16) as f32 * 8.0, 8.0, 8.0)),
                        dest_size: Some(vec2(s, s)),
                        ..Default::default()
                    },
                );
                draw_rectangle_lines(px, py, s, s, 2.0, WHITE);
                let t = self.doc.tile(x, y);
                draw_text(&format!("{x},{y}  #{t:02X}"), PANEL + 8.0, ch - 8.0, 20.0, WHITE);
            }
        }
        let (act, y) = self.world_panel_top();
        let act = match self.world_tab {
            TAB_PICTURE => {
                self.tile_palette(y);
                act
            }
            TAB_CUSTOM => self.custom_panel(y).unwrap_or(act),
            _ => {
                self.points_panel(y);
                act
            }
        };
        self.status_line(cw, ch);
        match act {
            1 => self.doc.world_undo(),
            2 => self.doc.world_redo(),
            3 => self.doc.restore_world(),
            4 => {
                self.save();
            }
            6 => self.play_map(),
            9 => self.enter_world(false),
            _ => {}
        }
    }

    /// Title, tabs, tools, undo and save. Returns the action chosen and where the panel
    /// continues.
    fn world_panel_top(&mut self) -> (u32, f32) {
        draw_rectangle(0.0, 0.0, PANEL, screen_height(), col(0));
        draw_rectangle(PANEL - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(tr(self.es, "WORLD MAP EDITOR", "EDITOR DEL MAPA"), 10.0, 22.0, 24.0, col(3));
        let mut y = 34.0;
        let mut act = 0;
        if button(Rect::new(10.0, y, 276.0, 26.0), tr(self.es, "Back to the levels", "Volver a los niveles"), false, 16.0) {
            act = 9;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 88.0, 26.0), tr(self.es, "Picture", "Dibujo"), self.world_tab == TAB_PICTURE, 15.0) {
            self.world_tab = TAB_PICTURE;
        }
        if button(Rect::new(102.0, y, 88.0, 26.0), tr(self.es, "Orig. points", "Puntos orig."), self.world_tab == TAB_POINTS, 15.0) {
            self.world_tab = TAB_POINTS;
        }
        if button(Rect::new(194.0, y, 92.0, 26.0), tr(self.es, "Your map", "Mapa propio"), self.world_tab == TAB_CUSTOM, 15.0) {
            self.world_tab = TAB_CUSTOM;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 92.0, 26.0), tr(self.es, "Pencil (B)", "Lapiz (B)"), self.tool.kind() == ToolKind::Pencil, 16.0) {
            self.set_tool(ToolKind::Pencil);
        }
        if button(Rect::new(106.0, y, 92.0, 26.0), tr(self.es, "Box (R)", "Caja (R)"), self.tool.kind() == ToolKind::Box, 16.0) {
            self.set_tool(ToolKind::Box);
        }
        if button(Rect::new(202.0, y, 84.0, 26.0), tr(self.es, "Grid (G)", "Rejilla (G)"), self.grid, 16.0) {
            self.grid = !self.grid;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 92.0, 26.0), tr(self.es, "Undo", "Deshacer"), false, 16.0) {
            act = 1;
        }
        if button(Rect::new(106.0, y, 92.0, 26.0), tr(self.es, "Redo", "Rehacer"), false, 16.0) {
            act = 2;
        }
        if button(Rect::new(202.0, y, 84.0, 26.0), tr(self.es, "Original", "Original"), false, 16.0) {
            act = 3;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 276.0, 28.0), tr(self.es, "Save (Ctrl+S)", "Guardar (Ctrl+S)"), false, 17.0) {
            act = 4;
        }
        y += 38.0;
        (act, y)
    }

    /// The world map's 256 tiles to paint with.
    fn tile_palette(&mut self, mut y: f32) {
        let mp: Vec2 = mouse_position().into();
        draw_text(&format!("{} #{:02X}", tr(self.es, "TILES  selected", "TILES  elegido"), self.tile), 10.0, y + 12.0, 18.0, col(3));
        y += 18.0;
        let Some(atlas) = &self.world_atlas else { return };
        let cell = (17.0f32).min((screen_height() - y - 30.0) / 16.0);
        for t in 0..=255u8 {
            let x = 10.0 + (t % 16) as f32 * cell;
            let yy = y + (t / 16) as f32 * cell;
            draw_texture_ex(atlas, x, yy, WHITE, DrawTextureParams {
                source: Some(Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0)),
                dest_size: Some(vec2(cell - 1.0, cell - 1.0)),
                ..Default::default()
            });
            if t == self.tile {
                draw_rectangle_lines(x - 1.0, yy - 1.0, cell + 1.0, cell + 1.0, 2.0, col(3));
            }
            if Rect::new(x, yy, cell - 1.0, cell - 1.0).contains(mp) && is_mouse_button_pressed(MouseButton::Left) {
                self.tile = t;
            }
        }
        draw_text(tr(self.es, "Right click on the map: pick a tile", "Clic derecho en el mapa: coger tile"), 10.0, (y + 16.0 * cell + 14.0).min(screen_height() - 4.0), 15.0, col(2));
    }

    /// Which level each point of the game's own map opens.
    fn points_panel(&mut self, mut y: f32) {
        draw_text(tr(self.es, "Point", "Punto"), 10.0, y + 18.0, 18.0, col(3));
        if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
            self.point = (self.point + world::POINT_COUNT - 1) % world::POINT_COUNT;
        }
        draw_text(&format!("{:02X}", self.point), 104.0, y + 18.0, 20.0, col(3));
        if button(Rect::new(130.0, y, 26.0, 26.0), "+", false, 18.0) {
            self.point = (self.point + 1) % world::POINT_COUNT;
        }
        y += 30.0;
        let cur = self.doc.points[self.point];
        let label = if cur == world::NO_LEVEL { tr(self.es, "no level", "sin nivel").to_string() } else { format!("{} {:02}", tr(self.es, "level", "nivel"), cur) };
        draw_text(tr(self.es, "Opens", "Abre"), 10.0, y + 18.0, 18.0, col(3));
        let mut change: i32 = 0;
        if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
            change = -1;
        }
        draw_text(&label, 104.0, y + 18.0, 18.0, col(3));
        if button(Rect::new(210.0, y, 26.0, 26.0), "+", false, 18.0) {
            change = 1;
        }
        self.doc.step_point_level(self.point, change);
        y += 34.0;
        draw_text(tr(self.es, "Points that open a level (click to choose):", "Puntos que abren un nivel (clic para elegir):"), 10.0, y + 12.0, 15.0, col(2));
        y += 18.0;
        let list: Vec<usize> = (0..world::POINT_COUNT).filter(|&p| self.doc.points[p] < 32).collect();
        for (k, &p) in list.iter().enumerate() {
            let r = Rect::new(10.0 + (k % 3) as f32 * 92.0, y + (k / 3) as f32 * 24.0, 88.0, 22.0);
            if button(r, &format!("{:02X}>{:02}", p, self.doc.points[p]), p == self.point, 15.0) {
                self.point = p;
            }
        }
        let yy = y + ((list.len() + 2) / 3) as f32 * 24.0 + 16.0;
        let note = tr(self.es, "Where each point sits and the paths between them are fixed in the game's code.", "La posicion de cada punto y los caminos estan fijos en el codigo del juego.");
        for (i, line) in wrap(note, 36).iter().enumerate() {
            draw_text(line, 10.0, yy + i as f32 * 16.0, 15.0, col(2));
        }
    }
}
