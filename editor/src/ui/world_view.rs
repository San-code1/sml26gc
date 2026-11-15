//! The world map editor: the world map and the zone maps, each picture painted tile by tile
//! (Picture tab), and the game's own points drawn where the game shows them with the level
//! each one opens (Orig. points tab). The third tab, the editor-made map, is in
//! `custom_view`.
use super::widgets::*;
use super::{left_button, Editor, WorldCanvas, TAB_CUSTOM, TAB_PICTURE, TAB_POINTS};
use crate::domain::document::EditorDocument;
use crate::domain::tools::ToolKind;
use cart_model::original_points::{self, OnMap, Spot};
use cart_model::world;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;

/// The maps in the order the map selector steps through them.
fn all_maps() -> Vec<OnMap> {
    std::iter::once(OnMap::World)
        .chain((0..ZONES.len()).map(OnMap::Zone))
        .collect()
}

impl Editor {
    /// Size of the shown map picture in pixels.
    fn map_px(&self) -> Vec2 {
        let (w, h) = EditorDocument::map_size(self.world_map);
        vec2(w as f32 * 8.0, h as f32 * 8.0)
    }

    fn map_name(&self, map: OnMap) -> &'static str {
        match map {
            OnMap::World => tr(self.es, "World map", "Mapa del mundo"),
            OnMap::Zone(z) => {
                if self.es {
                    ZONES[z].name_es
                } else {
                    ZONES[z].name
                }
            }
        }
    }

    /// Screen position of a pixel of the shown map picture.
    fn on_screen(&self, x: i32, y: i32) -> Vec2 {
        vec2(
            PANEL + (x as f32 - self.cam.x) * self.scale(),
            (y as f32 - self.cam.y) * self.scale(),
        )
    }

    /// The game's own point drawn under the mouse on the shown map.
    fn spot_at(&self, m: Vec2) -> Option<&'static Spot> {
        let w = self.world_pos(m);
        original_points::on(self.world_map)
            .find(|s| (vec2(s.x as f32, s.y as f32) - w).length() <= 7.0)
    }

    /// Choose one of the game's own points; the map it is drawn on is shown.
    fn select_original_point(&mut self, p: usize) {
        self.point = p;
        if let Some(s) = original_points::SPOTS
            .iter()
            .find(|s| s.point as usize == p)
        {
            self.show_map(s.map);
        }
    }

    /// Tile under the mouse on the shown map.
    pub(super) fn world_cell(&self, m: Vec2) -> Option<(usize, usize)> {
        if m.x < PANEL || m.y > screen_height() - STATUS_H {
            return None;
        }
        let w = self.world_pos(m);
        let (x, y) = ((w.x / 8.0).floor() as i32, (w.y / 8.0).floor() as i32);
        let (mw, mh) = EditorDocument::map_size(self.world_map);
        if x >= 0 && y >= 0 && (x as usize) < mw && (y as usize) < mh {
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
        let (space, _, _) = self.navigate(m, ctrl, self.map_px());
        if self.pan.is_some() || space {
            return;
        }
        if self.world_tab == TAB_CUSTOM {
            self.update_custom(m);
            return;
        }
        if self.world_tab == TAB_POINTS {
            if is_mouse_button_pressed(MouseButton::Left) && m.x >= PANEL {
                if let Some(s) = self.spot_at(m) {
                    self.point = s.point as usize;
                }
            }
            return;
        }
        let cell = self.world_cell(m);
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some((x, y)) = cell {
                self.tile = self.doc.tile(self.world_map, x, y);
            }
        }
        let mut canvas = WorldCanvas {
            doc: &mut self.doc,
            map: self.world_map,
            tile: self.tile,
        };
        self.tool.handle(
            &mut canvas,
            cell.map(|(x, y)| (x as i32, y as i32)),
            left_button(),
        );
    }

    pub(super) fn draw_world(&mut self) {
        clear_background(col(3));
        let s = self.scale() * 8.0;
        let (cw, ch) = (screen_width() - PANEL, screen_height() - STATUS_H);
        let (mw, mh) = EditorDocument::map_size(self.world_map);
        if let Some(atlas) = self.map_atlas() {
            let x0 = (self.cam.x / 8.0).floor() as i32;
            let y0 = (self.cam.y / 8.0).floor() as i32;
            let nx = (cw / s).ceil() as i32 + 1;
            let ny = (ch / s).ceil() as i32 + 1;
            for y in y0..(y0 + ny).min(mh as i32) {
                for x in x0..(x0 + nx).min(mw as i32) {
                    let t = self.doc.tile(self.world_map, x as usize, y as usize);
                    let p = self.on_screen(x * 8, y * 8);
                    draw_texture_ex(
                        atlas,
                        p.x.floor(),
                        p.y.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new(
                                (t % 16) as f32 * 8.0,
                                (t / 16) as f32 * 8.0,
                                8.0,
                                8.0,
                            )),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            let end = self.on_screen(mw as i32 * 8, mh as i32 * 8);
            if self.grid && s >= 12.0 && self.world_tab == TAB_PICTURE {
                let gc = Color::new(0.0, 0.0, 0.0, 0.12);
                for x in x0..=(x0 + nx).min(mw as i32) {
                    let px = self.on_screen(x * 8, 0).x;
                    draw_line(px, 0.0, px, end.y.min(ch), 1.0, gc);
                }
                for y in y0..=(y0 + ny).min(mh as i32) {
                    let py = self.on_screen(0, y * 8).y;
                    draw_line(PANEL, py, end.x.min(screen_width()), py, 1.0, gc);
                }
            }
            let m: Vec2 = mouse_position().into();
            if self.world_tab == TAB_CUSTOM {
                self.draw_custom();
            } else if self.world_tab == TAB_POINTS {
                self.draw_original_points();
            } else if let Some((x, y)) = self.world_cell(m) {
                let p = self.on_screen(x as i32 * 8, y as i32 * 8);
                draw_texture_ex(
                    atlas,
                    p.x,
                    p.y,
                    Color::new(1.0, 1.0, 1.0, 0.6),
                    DrawTextureParams {
                        source: Some(Rect::new(
                            (self.tile % 16) as f32 * 8.0,
                            (self.tile / 16) as f32 * 8.0,
                            8.0,
                            8.0,
                        )),
                        dest_size: Some(vec2(s, s)),
                        ..Default::default()
                    },
                );
                draw_rectangle_lines(p.x, p.y, s, s, 2.0, WHITE);
                let t = self.doc.tile(self.world_map, x, y);
                draw_text(
                    &format!("{x},{y}  #{t:02X}"),
                    PANEL + 8.0,
                    ch - 8.0,
                    20.0,
                    WHITE,
                );
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

    /// Title, map choice, tabs, tools, undo and save. Returns the action chosen and where
    /// the panel continues.
    fn world_panel_top(&mut self) -> (u32, f32) {
        draw_rectangle(0.0, 0.0, PANEL, screen_height(), col(0));
        draw_rectangle(PANEL - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(
            tr(self.es, "WORLD MAP EDITOR", "EDITOR DEL MAPA"),
            10.0,
            22.0,
            24.0,
            col(3),
        );
        let mut y = 34.0;
        let mut act = 0;
        if button(
            Rect::new(10.0, y, 276.0, 26.0),
            tr(self.es, "Back to the levels", "Volver a los niveles"),
            false,
            16.0,
        ) {
            act = 9;
        }
        y += 30.0;
        // which map: the world map or a zone map (your own map is drawn on the world map)
        let maps = all_maps();
        let i = maps.iter().position(|&m| m == self.world_map).unwrap_or(0);
        let fixed = self.world_tab == TAB_CUSTOM;
        if button(Rect::new(10.0, y, 30.0, 26.0), "<", false, 18.0) && !fixed {
            self.show_map(maps[(i + maps.len() - 1) % maps.len()]);
        }
        let name = self.map_name(self.world_map);
        let d = measure_text(name, None, 18, 1.0);
        draw_text(name, 148.0 - d.width / 2.0, y + 18.0, 18.0, col(3));
        if button(Rect::new(256.0, y, 30.0, 26.0), ">", false, 18.0) && !fixed {
            self.show_map(maps[(i + 1) % maps.len()]);
        }
        y += 30.0;
        if button(
            Rect::new(10.0, y, 88.0, 26.0),
            tr(self.es, "Picture", "Dibujo"),
            self.world_tab == TAB_PICTURE,
            15.0,
        ) {
            self.world_tab = TAB_PICTURE;
        }
        if button(
            Rect::new(102.0, y, 88.0, 26.0),
            tr(self.es, "Orig. points", "Puntos orig."),
            self.world_tab == TAB_POINTS,
            15.0,
        ) {
            self.world_tab = TAB_POINTS;
        }
        if button(
            Rect::new(194.0, y, 92.0, 26.0),
            tr(self.es, "Your map", "Mapa propio"),
            self.world_tab == TAB_CUSTOM,
            15.0,
        ) {
            self.world_tab = TAB_CUSTOM;
            self.show_map(OnMap::World);
        }
        y += 30.0;
        if button(
            Rect::new(10.0, y, 92.0, 26.0),
            tr(self.es, "Pencil (B)", "Lapiz (B)"),
            self.tool.kind() == ToolKind::Pencil,
            16.0,
        ) {
            self.set_tool(ToolKind::Pencil);
        }
        if button(
            Rect::new(106.0, y, 92.0, 26.0),
            tr(self.es, "Box (R)", "Caja (R)"),
            self.tool.kind() == ToolKind::Box,
            16.0,
        ) {
            self.set_tool(ToolKind::Box);
        }
        if button(
            Rect::new(202.0, y, 84.0, 26.0),
            tr(self.es, "Grid (G)", "Rejilla (G)"),
            self.grid,
            16.0,
        ) {
            self.grid = !self.grid;
        }
        y += 30.0;
        if button(
            Rect::new(10.0, y, 92.0, 26.0),
            tr(self.es, "Undo", "Deshacer"),
            false,
            16.0,
        ) {
            act = 1;
        }
        if button(
            Rect::new(106.0, y, 92.0, 26.0),
            tr(self.es, "Redo", "Rehacer"),
            false,
            16.0,
        ) {
            act = 2;
        }
        if button(
            Rect::new(202.0, y, 84.0, 26.0),
            tr(self.es, "Original", "Original"),
            false,
            16.0,
        ) {
            act = 3;
        }
        y += 30.0;
        if button(
            Rect::new(10.0, y, 276.0, 28.0),
            tr(self.es, "Save (Ctrl+S)", "Guardar (Ctrl+S)"),
            false,
            17.0,
        ) {
            act = 4;
        }
        y += 38.0;
        (act, y)
    }

    /// The shown map's 256 tiles to paint with.
    fn tile_palette(&mut self, mut y: f32) {
        let mp: Vec2 = mouse_position().into();
        draw_text(
            &format!(
                "{} #{:02X}",
                tr(self.es, "TILES  selected", "TILES  elegido"),
                self.tile
            ),
            10.0,
            y + 12.0,
            18.0,
            col(3),
        );
        y += 18.0;
        let Some(atlas) = self.map_atlas() else {
            return;
        };
        let cell = (17.0f32).min((screen_height() - y - 30.0) / 16.0);
        let mut picked = None;
        for t in 0..=255u8 {
            let x = 10.0 + (t % 16) as f32 * cell;
            let yy = y + (t / 16) as f32 * cell;
            draw_texture_ex(
                atlas,
                x,
                yy,
                WHITE,
                DrawTextureParams {
                    source: Some(Rect::new(
                        (t % 16) as f32 * 8.0,
                        (t / 16) as f32 * 8.0,
                        8.0,
                        8.0,
                    )),
                    dest_size: Some(vec2(cell - 1.0, cell - 1.0)),
                    ..Default::default()
                },
            );
            if t == self.tile {
                draw_rectangle_lines(x - 1.0, yy - 1.0, cell + 1.0, cell + 1.0, 2.0, col(3));
            }
            if Rect::new(x, yy, cell - 1.0, cell - 1.0).contains(mp)
                && is_mouse_button_pressed(MouseButton::Left)
            {
                picked = Some(t);
            }
        }
        if let Some(t) = picked {
            self.tile = t;
        }
        draw_text(
            tr(
                self.es,
                "Right click on the map: pick a tile",
                "Clic derecho en el mapa: coger tile",
            ),
            10.0,
            (y + 16.0 * cell + 14.0).min(screen_height() - 4.0),
            15.0,
            col(2),
        );
    }

    /// The game's own points on the shown map: where the game draws them, the level each
    /// opens, the chosen one ringed.
    fn draw_original_points(&self) {
        let r = (4.0 * self.scale()).max(6.0);
        for sp in original_points::on(self.world_map) {
            let c = self.on_screen(sp.x, sp.y);
            let level = self.doc.points[sp.point as usize];
            if level < 32 {
                let fs = (r * 1.3).clamp(13.0, 22.0);
                draw_circle(c.x, c.y, r + 2.0, BLACK);
                draw_circle(c.x, c.y, r, Color::new(0.95, 0.45, 0.1, 1.0));
                let t = format!("{level:02}");
                let d = measure_text(&t, None, fs as u16, 1.0);
                draw_text(&t, c.x - d.width / 2.0, c.y + d.offset_y / 2.0, fs, WHITE);
            } else {
                draw_circle(c.x, c.y, r * 0.6 + 1.5, BLACK);
                draw_circle(c.x, c.y, r * 0.6, Color::new(0.8, 0.8, 0.8, 1.0));
            }
            let tag = format!("{:02X}", sp.point);
            draw_text(&tag, c.x + r + 3.0, c.y - r * 0.4 + 1.0, 15.0, BLACK);
            draw_text(&tag, c.x + r + 2.0, c.y - r * 0.4, 15.0, WHITE);
            if sp.point as usize == self.point {
                draw_circle_lines(c.x, c.y, r + 5.0, 3.0, WHITE);
            }
        }
    }

    /// Which level each point of the game's own map opens.
    fn points_panel(&mut self, mut y: f32) {
        draw_text(tr(self.es, "Point", "Punto"), 10.0, y + 18.0, 18.0, col(3));
        if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
            self.select_original_point((self.point + world::POINT_COUNT - 1) % world::POINT_COUNT);
        }
        draw_text(
            &format!("{:02X}", self.point),
            104.0,
            y + 18.0,
            20.0,
            col(3),
        );
        if button(Rect::new(130.0, y, 26.0, 26.0), "+", false, 18.0) {
            self.select_original_point((self.point + 1) % world::POINT_COUNT);
        }
        let drawn = original_points::SPOTS
            .iter()
            .find(|s| s.point as usize == self.point)
            .map(|s| self.map_name(s.map));
        draw_text(
            drawn.unwrap_or(tr(self.es, "(not drawn)", "(no se dibuja)")),
            162.0,
            y + 18.0,
            15.0,
            col(2),
        );
        y += 30.0;
        let cur = self.doc.points[self.point];
        let label = if cur == world::NO_LEVEL {
            tr(self.es, "no level", "sin nivel").to_string()
        } else {
            format!("{} {:02}", tr(self.es, "level", "nivel"), cur)
        };
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
        let title = format!(
            "{} {}:",
            tr(self.es, "Points on", "Puntos en"),
            self.map_name(self.world_map)
        );
        draw_text(&title, 10.0, y + 12.0, 15.0, col(2));
        y += 18.0;
        let list: Vec<usize> = original_points::on(self.world_map)
            .map(|s| s.point as usize)
            .collect();
        for (k, &p) in list.iter().enumerate() {
            let r = Rect::new(
                10.0 + (k % 3) as f32 * 92.0,
                y + (k / 3) as f32 * 24.0,
                88.0,
                22.0,
            );
            let l = self.doc.points[p];
            let text = if l < 32 {
                format!("{p:02X} > {l:02}")
            } else {
                format!("{p:02X} > -")
            };
            if button(r, &text, p == self.point, 15.0) {
                self.point = p;
            }
        }
        let yy = y + list.len().div_ceil(3) as f32 * 24.0 + 16.0;
        let note = tr(
            self.es,
            "Click a point on the map to choose it. Where the points sit and the paths between them are fixed in the game's code; to move points or add new ones, use Your map.",
            "Haz clic en un punto del mapa para elegirlo. Donde estan los puntos y sus caminos esta fijo en el codigo del juego; para moverlos o anadir nuevos, usa Mapa propio.",
        );
        for (i, line) in wrap(note, 36).iter().enumerate() {
            draw_text(line, 10.0, yy + i as f32 * 16.0, 15.0, col(2));
        }
    }
}
