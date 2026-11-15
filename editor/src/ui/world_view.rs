//! The world map editor: the world map and the zone maps, each picture painted tile by tile
//! (Picture tab), and the game's own points drawn where the game shows them with the level
//! each one opens and the paths Mario walks between them (Points tab): levels, paths and new
//! points can be changed there.
use super::shortcuts::{keys, with_keys, Action};
use super::widgets::*;
use super::{Editor, TAB_PICTURE, TAB_POINTS};
use crate::domain::document::EditorDocument;
use crate::domain::tools::ToolKind;
use cart_model::doors::{self, Leads};
use cart_model::map_routes::{self, DIRS};
use cart_model::original_points::{self, OnMap};
use cart_model::world;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;

/// The maps in the order the map selector steps through them.
fn all_maps() -> Vec<OnMap> {
    std::iter::once(OnMap::World).chain((0..ZONES.len()).map(OnMap::Zone)).collect()
}

impl Editor {
    /// Size of the shown map picture in pixels.
    fn map_px(&self) -> Vec2 {
        let (w, h) = EditorDocument::map_size(self.world_map);
        vec2(w as f32 * 8.0, h as f32 * 8.0)
    }

    pub(super) fn map_name(&self, map: OnMap) -> &'static str {
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
        vec2(self.panel + (x as f32 - self.cam.x) * self.scale(), (y as f32 - self.cam.y) * self.scale())
    }

    /// The points drawn on the shown map: the game's own (where it draws them) and, on the
    /// world map, the new ones. (point, x, y, new)
    fn map_points(&self) -> Vec<(u8, i32, i32, bool)> {
        let mut v: Vec<(u8, i32, i32, bool)> = original_points::on(self.world_map).map(|s| (s.point, s.x, s.y, false)).collect();
        if self.world_map == OnMap::World {
            for &p in &self.doc.routes.added {
                let (x, y) = self.doc.routes.places[p as usize].on_world();
                v.push((p, x, y, true));
            }
        }
        v
    }

    /// The point drawn under the mouse on the shown map.
    fn point_at(&self, m: Vec2) -> Option<(u8, i32, i32, bool)> {
        let w = self.world_pos(m);
        self.map_points().into_iter().find(|s| (vec2(s.1 as f32, s.2 as f32) - w).length() <= 7.0)
    }

    /// Where the chosen point is drawn on the shown map, if it is.
    fn chosen_at(&self) -> Option<(i32, i32)> {
        self.map_points().into_iter().find(|s| s.0 as usize == self.point).map(|s| (s.1, s.2))
    }

    /// Is `p` a point of the world map (drawn there, the twin of a zone's entrance, or new)?
    fn is_world_point(&self, p: u8) -> bool {
        self.doc.routes.added.contains(&p) || map_routes::ZONE_EXITS.iter().any(|z| z.1 == p) || original_points::SPOTS.iter().any(|s| s.point == p && s.map == OnMap::World)
    }

    /// Where an exit can lead: every world map point, then every zone a door can enter.
    fn exit_targets(&self) -> Vec<Leads> {
        let mut pts: Vec<u8> = (0..map_routes::POINTS as u8).filter(|&p| self.is_world_point(p)).collect();
        pts.sort();
        pts.into_iter().map(Leads::World).chain((0..doors::INTO.len()).map(Leads::Zone)).collect()
    }

    fn leads_name(&self, l: Leads) -> String {
        match l {
            Leads::World(p) => format!("{p:02X} ({})", tr(self.es, "world", "mundo")),
            Leads::Zone(i) => self.map_name(OnMap::Zone(doors::INTO[i].zone)).to_string(),
        }
    }

    /// The label of a point on the map: its number and, for a door, where it goes.
    fn point_tag(&self, p: u8) -> Option<String> {
        if self.world_map != OnMap::World && self.is_world_point(p) {
            // a world point drawn where Mario walks out of a zone
            return Some(format!("{p:02X} ({})", tr(self.es, "world", "mundo")));
        }
        if let Some(z) = self.doc.doors.door_of(p) {
            return Some(format!("{p:02X} > {}", self.map_name(OnMap::Zone(doors::INTO[z].zone))));
        }
        let i = self.doc.doors.exit_on(p)?;
        Some(format!("{p:02X} > {}", self.leads_name(self.doc.doors.exits[i].1)))
    }

    /// The exits of the shown zone that can be moved to the chosen point.
    fn exit_sites(&self) -> Vec<usize> {
        let p = self.point as u8;
        let OnMap::Zone(z) = self.world_map else { return Vec::new() };
        if self.doc.doors.into.is_empty() || self.doc.doors.exit_on(p).is_some() || self.is_world_point(p) || self.chosen_at().is_none() {
            return Vec::new();
        }
        (0..doors::EXITS.len()).filter(|&i| original_points::SPOTS.iter().any(|s| s.point == doors::EXITS[i].point && s.map == OnMap::Zone(z))).collect()
    }

    /// Move the `k`th exit of the shown zone to the chosen point.
    fn make_exit(&mut self, k: usize) {
        if let Some(&i) = self.exit_sites().get(k) {
            let leads = self.doc.doors.exits[i].1;
            self.doc.set_exit(i, self.point as u8, leads);
        }
    }

    /// The chosen point's door (a world point) or exit (a zone's): one step through where
    /// it can lead.
    fn door_step(&mut self, step: i32) {
        if self.doc.doors.into.is_empty() {
            return;
        }
        let p = self.point as u8;
        if let Some(i) = self.doc.doors.exit_on(p) {
            let (at, leads) = self.doc.doors.exits[i];
            let targets = self.exit_targets();
            let k = targets.iter().position(|&t| t == leads).unwrap_or(0) as i32;
            self.doc.set_exit(i, at, targets[(k + step).rem_euclid(targets.len() as i32) as usize]);
        } else if self.is_world_point(p) {
            // none, then each zone
            let count = doors::INTO.len() as i32 + 1;
            let k = self.doc.doors.door_of(p).map(|z| z as i32 + 1).unwrap_or(0);
            let n = (k + step).rem_euclid(count);
            self.doc.set_door(p, if n == 0 { None } else { Some(n as usize - 1) });
        }
    }

    /// A `-` value `+` row with the keys of its two actions beside its name. Returns the
    /// action clicked.
    fn stepper(&self, y: f32, name: &str, value: &str, plus_x: f32, (minus, plus): (Action, Action)) -> Option<Action> {
        draw_text(name, 10.0, y + 18.0, 18.0, col(3));
        let k = format!("{} {}", keys(minus), keys(plus));
        draw_text(&k, 10.0, y + 32.0, 15.0, col(2));
        let mut act = None;
        if button(Rect::new(70.0, y, 26.0, 26.0), "-", false, 18.0) {
            act = Some(minus);
        }
        draw_text(value, 104.0, y + 18.0, 16.0, col(3));
        if button(Rect::new(plus_x, y, 26.0, 26.0), "+", false, 18.0) {
            act = Some(plus);
        }
        act
    }

    /// The doors part of the points panel: where the chosen point leads.
    fn door_rows(&mut self, mut y: f32) -> (f32, Option<Action>) {
        if self.doc.doors.into.is_empty() {
            return (y, None);
        }
        let p = self.point as u8;
        let mut act = None;
        if let Some(i) = self.doc.doors.exit_on(p) {
            let leads = self.doc.doors.exits[i].1;
            act = self.stepper(y, tr(self.es, "Exit to", "Sale a"), &self.leads_name(leads), 260.0, (Action::PrevDoor, Action::NextDoor));
            y += 34.0;
        } else if self.is_world_point(p) {
            let name = match self.doc.doors.door_of(p) {
                Some(z) => format!("{} {}", tr(self.es, "into", "entra en"), self.map_name(OnMap::Zone(doors::INTO[z].zone))),
                None => tr(self.es, "none", "ninguna").to_string(),
            };
            act = self.stepper(y, tr(self.es, "Door", "Puerta"), &name, 260.0, (Action::PrevDoor, Action::NextDoor));
            y += 34.0;
        } else {
            for (k, i) in self.exit_sites().into_iter().enumerate().take(2) {
                let at = self.doc.doors.exits[i].0;
                let a = if k == 0 { Action::MakeExit1 } else { Action::MakeExit2 };
                let label = format!("{} ({} {at:02X})", tr(self.es, "Make it the exit", "Hacer salida aqui"), tr(self.es, "now", "ahora"));
                if button(Rect::new(10.0, y, 276.0, 26.0), &with_keys(&label, a), false, 15.0) {
                    act = Some(a);
                }
                y += 30.0;
            }
        }
        (y, act)
    }

    /// Show the map `step` places further in the map selector.
    pub(super) fn step_map(&mut self, step: i32) {
        let maps = all_maps();
        let i = maps.iter().position(|&m| m == self.world_map).unwrap_or(0) as i32;
        self.show_map(maps[(i + step).rem_euclid(maps.len() as i32) as usize]);
    }

    /// The Points tab's actions.
    pub(super) fn run_points(&mut self, a: Action) {
        let n = world::POINT_COUNT;
        match a {
            Action::PrevPoint => self.select_original_point((self.point + n - 1) % n),
            Action::NextPoint => self.select_original_point((self.point + 1) % n),
            Action::PrevOpens => self.doc.step_point_level(self.point, -1),
            Action::NextOpens => self.doc.step_point_level(self.point, 1),
            Action::PrevDoor => self.door_step(-1),
            Action::NextDoor => self.door_step(1),
            Action::MakeExit1 => self.make_exit(0),
            Action::MakeExit2 => self.make_exit(1),
            Action::DeletePathRight | Action::DeletePathLeft | Action::DeletePathUp | Action::DeletePathDown => {
                let d = [Action::DeletePathRight, Action::DeletePathLeft, Action::DeletePathUp, Action::DeletePathDown].iter().position(|&x| x == a).unwrap_or(0);
                if self.point < map_routes::POINTS {
                    self.doc.set_way(self.point, d, None);
                }
            }
            Action::NewPath => {
                let drawing = self.route_draw.is_some();
                let can_draw = self.point < map_routes::POINTS && self.chosen_at().is_some();
                self.placing_point = false;
                self.route_draw = if drawing || !can_draw { None } else { Some(Vec::new()) };
                if !can_draw {
                    self.status = tr(self.es, "Choose a point on the map shown first.", "Elige antes un punto del mapa que se ve.").into();
                }
            }
            Action::NewPoint if self.world_map == OnMap::World => {
                self.route_draw = None;
                self.placing_point = !self.placing_point;
            }
            Action::DeletePoint if self.doc.routes.added.contains(&(self.point as u8)) => {
                self.doc.remove_point(self.point as u8);
            }
            _ => {}
        }
    }

    /// Choose a point and show its map (for the command line).
    pub fn choose_point(&mut self, p: usize) {
        self.select_original_point(p);
    }

    /// Choose a point; the map it is drawn on is shown.
    fn select_original_point(&mut self, p: usize) {
        self.point = p;
        self.route_draw = None;
        if self.doc.routes.added.contains(&(p as u8)) {
            self.show_map(OnMap::World);
        } else if let Some(s) = original_points::SPOTS.iter().find(|s| s.point as usize == p) {
            self.show_map(s.map);
        }
    }

    /// The mouse on the picture, on the 4-pixel steps paths are made of, counted from `from`.
    fn snapped(&self, m: Vec2, from: (i32, i32)) -> (i32, i32) {
        let w = self.world_pos(m);
        let s = |v: f32, o: i32| o + ((v - o as f32) / 4.0).round() as i32 * 4;
        (s(w.x, from.0), s(w.y, from.1))
    }

    /// Clicks on the map in the Points tab: choose a point, put a new one, or draw a path.
    fn points_clicks(&mut self, m: Vec2) {
        let clicked = is_mouse_button_pressed(MouseButton::Left) && m.x >= self.panel && m.y < screen_height() - STATUS_H;
        if self.placing_point {
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                self.placing_point = false;
            } else if clicked {
                // world points sit 2 pixels below the 4-pixel grid
                let (x, y) = self.snapped(m, (0, 2));
                self.placing_point = false;
                self.status = match self.doc.add_point(x, y) {
                    Some(p) => {
                        self.point = p as usize;
                        format!("{} {p:02X}", tr(self.es, "New point", "Punto nuevo"))
                    }
                    None => tr(self.es, "No free point numbers left.", "No quedan numeros de punto libres.").into(),
                };
            }
            return;
        }
        if let Some(corners) = &mut self.route_draw {
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                self.route_draw = None;
                return;
            }
            if is_key_pressed(KeyCode::Backspace) {
                corners.pop();
            }
            if !clicked {
                return;
            }
            let Some(start) = self.chosen_at() else {
                self.route_draw = None;
                return;
            };
            match self.point_at(m) {
                Some((p, x, y, _)) if p as usize != self.point && (p as usize) < map_routes::POINTS => {
                    let mut rel: Vec<(i32, i32)> = self.route_draw.take().unwrap_or_default().iter().map(|c| (c.0 - start.0, c.1 - start.1)).collect();
                    rel.push((x - start.0, y - start.1));
                    self.status = match self.doc.draw_path(self.point, p, &rel) {
                        Some((d, back)) => {
                            let back = back.map(|b| format!(", {} {p:02X} {} > {:02X}", tr(self.es, "and", "y"), DIRS[b], self.point)).unwrap_or_default();
                            format!("{} {:02X} {} > {p:02X}{back}", tr(self.es, "Path", "Camino"), self.point, DIRS[d])
                        }
                        None => tr(self.es, "That path has no steps.", "Ese camino no tiene pasos.").into(),
                    };
                }
                Some(_) => {}
                None => {
                    let c = self.snapped(m, start);
                    if let Some(corners) = &mut self.route_draw {
                        corners.push(c);
                    }
                }
            }
            return;
        }
        if clicked {
            if let Some(s) = self.point_at(m) {
                self.point = s.0 as usize;
            }
        }
    }

    /// Tile under the mouse on the shown map.
    pub(super) fn world_cell(&self, m: Vec2) -> Option<(usize, usize)> {
        if m.x < self.panel || m.y > screen_height() - STATUS_H {
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
        let (space, _, _) = self.navigate(m, ctrl, self.map_px());
        if self.pan.is_some() || space {
            return;
        }
        if self.world_tab == TAB_POINTS {
            self.points_clicks(m);
            return;
        }
        let cell = self.world_cell(m).map(|(x, y)| (x as i32, y as i32));
        self.edit_grid(cell);
    }

    pub(super) fn draw_world(&mut self) {
        clear_background(col(3));
        let s = self.scale() * 8.0;
        let (cw, ch) = (screen_width() - self.panel, screen_height() - STATUS_H);
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
                            source: Some(Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0)),
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
                    draw_line(self.panel, py, end.x.min(screen_width()), py, 1.0, gc);
                }
            }
            let m: Vec2 = mouse_position().into();
            if self.world_tab == TAB_POINTS {
                self.draw_original_points();
            } else {
                let cell = self.world_cell(m);
                self.draw_grid_overlay(atlas, cell.map(|(x, y)| (x as i32, y as i32)));
                if let Some((x, y)) = cell {
                    let t = self.doc.tile(self.world_map, x, y);
                    draw_text(&format!("{x},{y}  #{t:02X}"), self.panel + 8.0, ch - 8.0, 20.0, WHITE);
                }
            }
        }
        let (act, y) = self.world_panel_top();
        let act = match self.world_tab {
            TAB_PICTURE => {
                self.tile_palette(y);
                act
            }
            _ => self.points_panel(y).or(act),
        };
        self.status_line(cw, ch);
        if let Some(a) = act {
            self.run(a);
        }
    }

    /// Undo in the tab shown: the map pictures, or the points (levels, paths, doors).
    pub(super) fn world_undo(&mut self) {
        let done = match self.world_tab {
            TAB_PICTURE => {
                self.doc.world_undo();
                true
            }
            _ => self.doc.points_undo(),
        };
        if !done {
            self.status = tr(self.es, "Nothing to undo.", "Nada que deshacer.").into();
        }
    }

    pub(super) fn world_redo(&mut self) {
        let done = match self.world_tab {
            TAB_PICTURE => {
                self.doc.world_redo();
                true
            }
            _ => self.doc.points_redo(),
        };
        if !done {
            self.status = tr(self.es, "Nothing to redo.", "Nada que rehacer.").into();
        }
    }

    /// Title, map choice, tabs, tools, undo and save. Returns the action chosen and where
    /// the panel continues.
    fn world_panel_top(&mut self) -> (Option<Action>, f32) {
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        draw_text(tr(self.es, "WORLD MAP EDITOR", "EDITOR DEL MAPA"), 10.0, 22.0, 24.0, col(3));
        let es = self.es;
        let mut y = 34.0;
        let mut act = None;
        let mut b = |r: Rect, name: &str, a: Action, on: bool| {
            if button(r, &with_keys(name, a), on, 16.0) {
                act = Some(a);
            }
        };
        let (w2, w3) = (136.0, 89.0);
        let col2 = |i: usize| 10.0 + i as f32 * (w2 + 4.0);
        let col3 = |i: usize| 10.0 + i as f32 * (w3 + 4.0);
        b(Rect::new(10.0, y, 276.0, 26.0), tr(es, "Back to the levels", "Volver a los niveles"), Action::WorldEditor, false);
        y += 30.0;
        // which map: the world map or a zone map
        let name = self.map_name(self.world_map);
        let d = measure_text(name, None, 18, 1.0);
        draw_text(name, 148.0 - d.width / 2.0, y + 15.0, 18.0, col(3));
        let k = format!("{} / {}", keys(Action::PrevMap), keys(Action::NextMap));
        let d = measure_text(&k, None, 12, 1.0);
        draw_text(&k, 148.0 - d.width / 2.0, y + 27.0, 12.0, col(2));
        if button(Rect::new(10.0, y, 30.0, 28.0), "<", false, 18.0) {
            act = Some(Action::PrevMap);
        }
        if button(Rect::new(256.0, y, 30.0, 28.0), ">", false, 18.0) {
            act = Some(Action::NextMap);
        }
        y += 32.0;
        let picture = self.world_tab == TAB_PICTURE;
        if button(Rect::new(col3(0), y, w3, 26.0), &with_keys(tr(es, "Picture", "Dibujo"), Action::SwitchTab), picture, 15.0) {
            self.world_tab = TAB_PICTURE;
        }
        if button(Rect::new(col3(1), y, w3, 26.0), &with_keys(tr(es, "Points", "Puntos"), Action::SwitchTab), !picture, 15.0) {
            self.world_tab = TAB_POINTS;
        }
        let mut b = |r: Rect, name: &str, a: Action, on: bool| {
            if button(r, &with_keys(name, a), on, 16.0) {
                act = Some(a);
            }
        };
        b(Rect::new(col3(2), y, w3, 26.0), tr(es, "Play", "Jugar"), Action::Play, false);
        y += 30.0;
        let tool = self.tool.kind();
        if picture {
            b(Rect::new(col3(0), y, w3, 26.0), tr(es, "Pencil", "Lapiz"), Action::Pencil, tool == ToolKind::Pencil);
            b(Rect::new(col3(1), y, w3, 26.0), tr(es, "Box", "Caja"), Action::BoxFill, tool == ToolKind::Box);
            b(Rect::new(col3(2), y, w3, 26.0), tr(es, "Select", "Seleccion"), Action::Select, tool == ToolKind::Select);
            y += 30.0;
        }
        b(Rect::new(col2(0), y, w2, 26.0), tr(es, "Undo", "Deshacer"), Action::Undo, false);
        b(Rect::new(col2(1), y, w2, 26.0), tr(es, "Redo", "Rehacer"), Action::Redo, false);
        y += 30.0;
        if picture {
            b(Rect::new(col3(0), y, w3, 26.0), tr(es, "Grid", "Rejilla"), Action::Grid, self.grid);
            b(Rect::new(col3(1), y, 276.0 - w3 - 4.0, 26.0), "Original", Action::Original, false);
        } else {
            b(Rect::new(10.0, y, 276.0, 26.0), "Original", Action::Original, false);
        }
        y += 30.0;
        b(Rect::new(10.0, y, 276.0, 28.0), tr(es, "Save", "Guardar"), Action::Save, false);
        y += 36.0;
        (act, y)
    }

    /// The points on the shown map: where the game draws them, the level each opens, the
    /// paths Mario walks out of each (the chosen point's highlighted), the chosen one ringed.
    fn draw_original_points(&self) {
        let r = (4.0 * self.scale()).max(6.0);
        let pts = self.map_points();
        for &(p, x, y, _) in &pts {
            let Some(ways) = self.doc.routes.ways.get(p as usize) else { continue };
            if self.world_map != OnMap::World && self.is_world_point(p) {
                continue; // its paths are on the world map
            }
            let chosen = p as usize == self.point;
            for (d, w) in ways.iter().enumerate() {
                let Some(w) = w else { continue };
                let track: Vec<Vec2> = w.track().iter().map(|t| self.on_screen(x + t.0, y + t.1)).collect();
                let (c, th) = if chosen { (Color::new(1.0, 0.85, 0.1, 0.95), 3.0) } else { (Color::new(0.1, 0.1, 0.1, 0.75), 2.0) };
                for s in track.windows(2) {
                    draw_line(s[0].x, s[0].y, s[1].x, s[1].y, th, c);
                }
                if chosen && track.len() > 1 {
                    let a = track[(track.len() / 2).max(1)];
                    let t = format!("{} > {:02X}", DIRS[d], w.to);
                    draw_text(&t, a.x + 5.0, a.y - 3.0, 16.0, BLACK);
                    draw_text(&t, a.x + 4.0, a.y - 4.0, 16.0, Color::new(1.0, 0.95, 0.5, 1.0));
                }
            }
        }
        // the path being drawn, to the mouse
        if let (Some(corners), Some(start)) = (&self.route_draw, self.chosen_at()) {
            let m = self.snapped(mouse_position().into(), start);
            let mut line = vec![start];
            line.extend(corners.iter().copied());
            line.push(m);
            for s in line.windows(2) {
                let (a, b) = (self.on_screen(s[0].0, s[0].1), self.on_screen(s[1].0, s[1].1));
                draw_line(a.x, a.y, b.x, b.y, 3.0, WHITE);
            }
        }
        if self.placing_point {
            let (x, y) = self.snapped(mouse_position().into(), (0, 2));
            let c = self.on_screen(x, y);
            draw_circle_lines(c.x, c.y, r, 2.0, WHITE);
        }
        for (i, sp) in original_points::SPOTS.iter().enumerate() {
            if sp.map != self.world_map {
                continue;
            }
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
            let tag = match (self.point_tag(sp.point), original_points::door(i)) {
                (Some(t), _) => t,
                (None, Some(to)) if self.doc.doors.into.is_empty() => format!("{:02X} > {}", sp.point, self.map_name(to.map)),
                _ => format!("{:02X}", sp.point),
            };
            draw_text(&tag, c.x + r + 3.0, c.y - r * 0.4 + 1.0, 15.0, BLACK);
            draw_text(&tag, c.x + r + 2.0, c.y - r * 0.4, 15.0, WHITE);
            if sp.point as usize == self.point {
                draw_circle_lines(c.x, c.y, r + 5.0, 3.0, WHITE);
            }
        }
        // the new points
        for &(p, x, y, new) in &pts {
            if !new {
                continue;
            }
            let c = self.on_screen(x, y);
            let level = self.doc.points[p as usize];
            draw_circle(c.x, c.y, r + 2.0, BLACK);
            draw_circle(c.x, c.y, r, Color::new(0.2, 0.7, 0.3, 1.0));
            if (level as usize) < self.doc.levels.len() {
                let fs = (r * 1.3).clamp(13.0, 22.0);
                let t = format!("{level:02}");
                let d = measure_text(&t, None, fs as u16, 1.0);
                draw_text(&t, c.x - d.width / 2.0, c.y + d.offset_y / 2.0, fs, WHITE);
            }
            let tag = format!("{} {}", self.point_tag(p).unwrap_or(format!("{p:02X}")), tr(self.es, "(new)", "(nuevo)"));
            draw_text(&tag, c.x + r + 3.0, c.y - r * 0.4 + 1.0, 15.0, BLACK);
            draw_text(&tag, c.x + r + 2.0, c.y - r * 0.4, 15.0, WHITE);
            if p as usize == self.point {
                draw_circle_lines(c.x, c.y, r + 5.0, 3.0, WHITE);
            }
        }
    }

    /// Which level each point of the game's own map opens.
    fn points_panel(&mut self, mut y: f32) -> Option<Action> {
        let mut act = self.stepper(y, tr(self.es, "Point", "Punto"), &format!("{:02X}", self.point), 130.0, (Action::PrevPoint, Action::NextPoint));
        let new = self.doc.routes.added.contains(&(self.point as u8));
        let drawn = if new { Some(tr(self.es, "new point", "punto nuevo")) } else { original_points::SPOTS.iter().find(|s| s.point as usize == self.point).map(|s| self.map_name(s.map)) };
        draw_text(drawn.unwrap_or(tr(self.es, "(not drawn)", "(no se dibuja)")), 162.0, y + 18.0, 15.0, col(2));
        y += 34.0;
        let cur = self.doc.points[self.point];
        let label = match self.doc.point_level(self.point) {
            Some(l) if l >= cart_model::levels::LEVELS => format!("{} {l:02} ({} {:02})", tr(self.es, "level", "nivel"), tr(self.es, "new, as", "nuevo, como"), self.doc.base_of(l)),
            Some(l) => format!("{} {l:02}", tr(self.es, "level", "nivel")),
            None if cur == world::NO_LEVEL => tr(self.es, "no level", "sin nivel").to_string(),
            None => format!("{} ${cur:02X}", tr(self.es, "special", "especial")),
        };
        act = act.or(self.stepper(y, tr(self.es, "Opens", "Abre"), &label, 210.0, (Action::PrevOpens, Action::NextOpens)));
        y += 34.0;
        let (ny, door) = self.door_rows(y);
        y = ny;
        act = act.or(door);
        // the ways out of the point
        draw_text(tr(self.es, "Paths (pad > point); button: delete", "Caminos (cruceta > punto); boton: borrar"), 10.0, y + 12.0, 15.0, col(2));
        y += 18.0;
        if self.point < map_routes::POINTS {
            for d in 0..4 {
                let r = Rect::new(10.0 + (d % 2) as f32 * 138.0, y + (d / 2) as f32 * 26.0, 134.0, 24.0);
                match self.doc.routes.ways[self.point][d].as_ref().map(|w| w.to) {
                    Some(to) => {
                        draw_text(&format!("{} > {to:02X}", DIRS[d]), r.x + 4.0, r.y + 17.0, 17.0, col(3));
                        let a = [Action::DeletePathRight, Action::DeletePathLeft, Action::DeletePathUp, Action::DeletePathDown][d];
                        if button(Rect::new(r.x + 58.0, r.y, 76.0, 22.0), keys(a), false, 14.0) {
                            act = Some(a);
                        }
                    }
                    None => {
                        draw_text(&format!("{}  -", DIRS[d]), r.x + 4.0, r.y + 17.0, 17.0, col(2));
                    }
                }
            }
            y += 54.0;
        }
        if button(Rect::new(10.0, y, 134.0, 26.0), &with_keys(tr(self.es, "New path", "Nuevo camino"), Action::NewPath), self.route_draw.is_some(), 15.0) {
            act = Some(Action::NewPath);
        }
        if self.world_map == OnMap::World && button(Rect::new(148.0, y, 138.0, 26.0), &with_keys(tr(self.es, "New point", "Nuevo punto"), Action::NewPoint), self.placing_point, 15.0) {
            act = Some(Action::NewPoint);
        }
        y += 30.0;
        if new {
            if button(Rect::new(10.0, y, 134.0, 26.0), &with_keys(tr(self.es, "Delete point", "Borrar punto"), Action::DeletePoint), false, 15.0) {
                act = Some(Action::DeletePoint);
            }
            y += 30.0;
        }
        y += 4.0;
        let title = format!("{} {}:", tr(self.es, "Points on", "Puntos en"), self.map_name(self.world_map));
        draw_text(&title, 10.0, y + 12.0, 15.0, col(2));
        y += 18.0;
        let list: Vec<usize> = self.map_points().iter().map(|s| s.0 as usize).collect();
        for (k, &p) in list.iter().enumerate() {
            let r = Rect::new(10.0 + (k % 3) as f32 * 92.0, y + (k / 3) as f32 * 24.0, 88.0, 22.0);
            let l = self.doc.points[p];
            let text = if (l as usize) < self.doc.levels.len() { format!("{p:02X} > {l:02}") } else { format!("{p:02X} > -") };
            if button(r, &text, p == self.point, 15.0) {
                self.point = p;
            }
        }
        let yy = y + list.len().div_ceil(3) as f32 * 24.0 + 16.0;
        let note = if self.route_draw.is_some() {
            tr(
                self.es,
                "Drawing a path: click the corners, then the point it goes to. The pad direction is the way it starts; the way back is added too if that point has none that way. Backspace: last corner off. Esc or right click: cancel.",
                "Dibujando un camino: haz clic en las esquinas y luego en el punto al que va. La direccion de la cruceta es hacia donde empieza; tambien se crea la vuelta si ese punto no tiene camino en esa direccion. Retroceso: quitar la ultima esquina. Esc o clic derecho: cancelar.",
            )
        } else if self.placing_point {
            tr(self.es, "Click on the world map where the new point goes. Esc: cancel.", "Haz clic en el mapa del mundo donde va el punto nuevo. Esc: cancelar.")
        } else {
            tr(
                self.es,
                "Click a point on the map to choose it; with - + choose the level it opens. Door: a world point can enter a zone (A or START on it). Exit to: a zone's exit leads to a world point or straight into another zone; Make it the exit moves it to the chosen zone point. Paths are drawn in yellow; New path draws one, New point puts one on the world map (6 numbers free). Play (F5) saves and tries it.",
                "Haz clic en un punto del mapa para elegirlo; con - + elige el nivel que abre. Puerta: un punto del mundo puede entrar en una zona (A o START sobre el). Sale a: la salida de una zona lleva a un punto del mundo o directamente a otra zona; Hacer salida aqui la mueve al punto elegido de la zona. Los caminos se ven en amarillo; Nuevo camino dibuja uno, Nuevo punto pone uno en el mundo (6 numeros libres). Jugar (F5) guarda y lo prueba.",
            )
        };
        for (i, line) in wrap(note, 36).iter().enumerate() {
            draw_text(line, 10.0, yy + i as f32 * 16.0, 15.0, col(2));
        }
        act
    }
}
