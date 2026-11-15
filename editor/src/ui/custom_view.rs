//! "Your map": the editor-made world map over the world map and zone map pictures. Points
//! go anywhere, paths join them, each point may open a level; a path to a point on another
//! picture is a door. The game walks it with the d-pad.
use super::widgets::*;
use super::Editor;
use crate::domain::document::EditorDocument;
use cart_model::custom_map::Point;
use macroquad::prelude::*;

impl Editor {
    /// The point under a screen position (within 7 map pixels).
    fn point_at(&self, m: Vec2) -> Option<usize> {
        let w = self.world_pos(m);
        self.doc
            .custom
            .points
            .iter()
            .enumerate()
            .filter(|(_, p)| p.map == self.world_map)
            .map(|(i, p)| (i, (vec2(p.x as f32, p.y as f32) - w).length()))
            .filter(|(_, d)| *d <= 7.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| i)
    }

    pub(super) fn update_custom(&mut self, m: Vec2) {
        if self.world_cell(m).is_none() {
            return;
        }
        let w = self.world_pos(m);
        let on = self.world_map;
        let (mw, mh) = EditorDocument::map_size(on);
        let snap_x = |v: f32| ((v / 4.0).round() * 4.0).clamp(4.0, mw as f32 * 8.0 - 4.0) as i32;
        let snap_y = |v: f32| ((v / 4.0).round() * 4.0).clamp(4.0, mh as f32 * 8.0 - 4.0) as i32;
        if is_key_pressed(KeyCode::L) {
            self.link_from = self.sel_point;
        }
        if (is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace)) && self.sel_point.is_some() {
            self.doc.custom.remove_point(self.sel_point.unwrap());
            self.sel_point = None;
            self.doc.custom_dirty = true;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            match (self.point_at(m), self.link_from) {
                (Some(b), Some(a)) => {
                    // joining mode: the second point toggles a path
                    self.doc.custom.toggle_path(a, b);
                    self.link_from = None;
                    self.sel_point = Some(b);
                    self.doc.custom_dirty = true;
                }
                (Some(i), None) => {
                    self.sel_point = Some(i);
                    self.drag_point = true;
                }
                (None, _) => {
                    // a new point, joined to the selected one so a route is drawn click by click
                    let map = &mut self.doc.custom;
                    map.points.push(Point { x: snap_x(w.x), y: snap_y(w.y), level: None, map: on });
                    let n = map.points.len() - 1;
                    if let Some(s) = self.sel_point.filter(|&s| map.points[s].map == on) {
                        map.toggle_path(s, n);
                    }
                    self.sel_point = Some(n);
                    self.link_from = None;
                    self.drag_point = true;
                    self.doc.custom_dirty = true;
                }
            }
        }
        if self.drag_point {
            if is_mouse_button_down(MouseButton::Left) {
                if let Some(i) = self.sel_point {
                    let (x, y) = (snap_x(w.x), snap_y(w.y));
                    let p = &mut self.doc.custom.points[i];
                    if (p.x, p.y) != (x, y) {
                        p.x = x;
                        p.y = y;
                        self.doc.custom_dirty = true;
                    }
                }
            } else {
                self.drag_point = false;
            }
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            if let Some(i) = self.point_at(m) {
                self.doc.custom.remove_point(i);
                self.doc.custom_dirty = true;
            }
            self.sel_point = None;
            self.link_from = None;
        }
    }

    pub(super) fn draw_custom(&self) {
        let map = &self.doc.custom;
        let on = self.world_map;
        let to = |x: i32, y: i32| vec2(PANEL + (x as f32 - self.cam.x) * self.scale(), (y as f32 - self.cam.y) * self.scale());
        for &(a, b) in &map.paths {
            let (pa, pb) = (map.points[a], map.points[b]);
            if pa.map != on || pb.map != on {
                continue;
            }
            let (s, e) = (to(pa.x, pa.y), to(pb.x, pb.y));
            draw_line(s.x, s.y, e.x, e.y, 3.0 * self.scale().max(1.0), Color::new(0.12, 0.12, 0.12, 0.85));
        }
        if let Some(a) = self.link_from.filter(|&a| a < map.points.len()) {
            let p = map.points[a];
            let m: Vec2 = mouse_position().into();
            if p.map == on {
                let s = to(p.x, p.y);
                draw_line(s.x, s.y, m.x, m.y, 2.0, WHITE);
            } else {
                let t = format!("{} {}", tr(self.es, "joining from", "uniendo desde"), self.map_name(p.map));
                draw_text(&t, m.x + 12.0, m.y - 8.0, 18.0, WHITE);
            }
        }
        for (i, p) in map.points.iter().enumerate() {
            if p.map != on {
                continue;
            }
            let c = to(p.x, p.y);
            let r = 5.0 * self.scale();
            match p.level {
                Some(l) => {
                    draw_rectangle(c.x - r, c.y - r, r * 2.0, r * 2.0, col(3));
                    draw_rectangle(c.x - r + 2.0, c.y - r + 2.0, r * 2.0 - 4.0, r * 2.0 - 4.0, col(0));
                    let t = format!("{l:02}");
                    let fs = (r * 1.2).clamp(12.0, 22.0);
                    let d = measure_text(&t, None, fs as u16, 1.0);
                    draw_text(&t, c.x - d.width / 2.0, c.y + d.offset_y / 2.0, fs, col(3));
                }
                None => draw_circle(c.x, c.y, r * 0.5, col(3)),
            }
            if i == map.start {
                draw_text("S", c.x + r + 2.0, c.y - r, 18.0, WHITE);
            }
            if let Some(d) = map.door(i) {
                let t = format!("> {}", self.map_name(map.points[d].map));
                draw_text(&t, c.x + r + 3.0, c.y + r + 13.0, 16.0, BLACK);
                draw_text(&t, c.x + r + 2.0, c.y + r + 12.0, 16.0, WHITE);
            }
            if self.sel_point == Some(i) {
                draw_rectangle_lines(c.x - r - 3.0, c.y - r - 3.0, r * 2.0 + 6.0, r * 2.0 + 6.0, 2.0, WHITE);
            }
        }
    }

    /// The selected point's level, start, joining and deleting; trying the map in the game;
    /// help. Returns 6 when "Try it in the game" was clicked.
    pub(super) fn custom_panel(&mut self, mut y: f32) -> Option<u32> {
        let es = self.es;
        let mut act = None;
        draw_text(&format!("{} {}   {} {}", tr(es, "points:", "puntos:"), self.doc.custom.points.len(), tr(es, "paths:", "caminos:"), self.doc.custom.paths.len()), 10.0, y + 12.0, 17.0, col(3));
        y += 22.0;
        if let Some(i) = self.sel_point.filter(|&i| i < self.doc.custom.points.len()) {
            let cur = self.doc.custom.points[i].level;
            draw_text(&format!("{} {i}", tr(es, "Point", "Punto")), 10.0, y + 18.0, 18.0, col(3));
            if let Some(d) = self.doc.custom.door(i) {
                let t = format!("{} {}", tr(es, "door to", "puerta a"), self.map_name(self.doc.custom.points[d].map));
                draw_text(&t, 100.0, y + 18.0, 16.0, col(2));
            }
            y += 28.0;
            draw_text(tr(es, "Opens", "Abre"), 10.0, y + 18.0, 18.0, col(3));
            let mut change = 0i32;
            if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
                change = -1;
            }
            let label = match cur {
                Some(l) => format!("{} {:02}", tr(es, "level", "nivel"), l),
                None => tr(es, "nothing (junction)", "nada (cruce)").to_string(),
            };
            draw_text(&label, 104.0, y + 18.0, 17.0, col(3));
            if button(Rect::new(260.0, y, 26.0, 26.0), "+", false, 18.0) {
                change = 1;
            }
            if change != 0 {
                // cycle: nothing, 0, 1, ... 31, nothing
                let idx = cur.map(|l| l as i32).unwrap_or(32);
                let n = (idx + change).rem_euclid(33);
                self.doc.custom.points[i].level = if n == 32 { None } else { Some(n as u8) };
                self.doc.custom_dirty = true;
            }
            y += 32.0;
            if button(Rect::new(10.0, y, 136.0, 26.0), tr(es, "Mario starts here", "Mario empieza aqui"), self.doc.custom.start == i, 14.0) {
                self.doc.custom.start = i;
                self.doc.custom_dirty = true;
            }
            if button(Rect::new(150.0, y, 136.0, 26.0), tr(es, "Join to... (L)", "Unir con... (L)"), self.link_from == Some(i), 14.0) {
                self.link_from = if self.link_from == Some(i) { None } else { Some(i) };
            }
            y += 30.0;
            if button(Rect::new(10.0, y, 136.0, 26.0), tr(es, "Delete point", "Borrar punto"), false, 14.0) {
                self.doc.custom.remove_point(i);
                self.sel_point = None;
                self.doc.custom_dirty = true;
            }
            y += 34.0;
        } else {
            y += 4.0;
        }
        if button(Rect::new(10.0, y, 276.0, 28.0), tr(es, "Try it in the game", "Probarlo en el juego"), false, 16.0) {
            act = Some(6);
        }
        y += 32.0;
        if button(Rect::new(10.0, y, 276.0, 26.0), tr(es, "Add the game's own points", "Anadir los puntos originales"), false, 15.0) {
            let added = self.doc.add_original_points();
            self.status = format!("{} {added}", tr(es, "Original points added:", "Puntos originales anadidos:"));
        }
        y += 36.0;
        let help = tr(
            es,
            "Click empty map: new point, joined to the selected one. Drag: move. Right click: delete. Join to... then click another point: add or remove a path; change map with < > before the second click to make a door to a zone. In the game, the d-pad walks the paths, A or Start enters the point's level or goes through a door. The pause menu switches between this map and the original.",
            "Clic en el mapa vacio: punto nuevo, unido al elegido. Arrastrar: mover. Clic derecho: borrar. Unir con... y clic en otro punto: pone o quita un camino; cambia de mapa con < > antes del segundo clic para hacer una puerta a una zona. En el juego, la cruceta recorre los caminos, A o Start entra en el nivel del punto o cruza la puerta. El menu de pausa cambia entre este mapa y el original.",
        );
        for (k, line) in wrap(help, 38).iter().enumerate() {
            draw_text(line, 10.0, y + k as f32 * 16.0, 15.0, col(2));
        }
        act
    }
}
