//! The world map editor: the world map, the zone maps and the new zones, each picture painted
//! tile by tile (Picture tab), and the points drawn where the game shows them with the level
//! each one opens and the paths Mario walks between them (Points tab): levels, paths, doors,
//! exits and new points can be changed there. New zones are made from a zone and deleted here.
use super::shortcuts::{keys, with_keys, Action};
use super::widgets::*;
use super::names_view::Named;
use super::{Editor, TAB_PICTURE, TAB_POINTS};
use crate::domain::document::{EditorDocument, ZoneRefusal};
use crate::domain::tools::ToolKind;
use cart_model::added_points;
use cart_model::doors::{self, Leads};
use cart_model::map_routes::{self, Way, DIRS};
use cart_model::original_points::{self, OnMap};
use cart_model::world;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;

/// Mario's place on a zone map is this far below where the game draws its point.
const FEET: i32 = 4;

impl Editor {
    /// The maps in the order the map selector steps through them.
    fn all_maps(&self) -> Vec<OnMap> {
        std::iter::once(OnMap::World).chain((0..ZONES.len()).map(OnMap::Zone)).chain((0..self.doc.new_zones.len()).map(OnMap::NewZone)).collect()
    }

    /// The entry point of the game zone shown, if a door enters it.
    fn game_entry_shown(&self) -> Option<u8> {
        match self.world_map {
            OnMap::Zone(z) => self.doc.game_entry(z),
            _ => None,
        }
    }

    /// Can the chosen point be the entry of the game zone shown (one of its own points)?
    fn can_be_game_entry(&self) -> bool {
        match self.world_map {
            OnMap::Zone(z) => self.game_entry_shown().is_some() && cart_model::new_zones::base_of(z).is_some_and(|b| b.has(self.point as u8)) && self.chosen_at().is_some(),
            _ => false,
        }
    }

    /// The new zone shown, if one is.
    fn new_zone_shown(&self) -> Option<usize> {
        match self.world_map {
            OnMap::NewZone(k) if k < self.doc.new_zones.len() => Some(k),
            _ => None,
        }
    }

    /// Size of the shown map picture in pixels.
    fn map_px(&self) -> Vec2 {
        let (w, h) = EditorDocument::map_size(self.world_map);
        vec2(w as f32 * 8.0, h as f32 * 8.0)
    }

    pub(super) fn map_name(&self, map: OnMap) -> String {
        let zone = |z: usize| if self.es { ZONES[z].name_es } else { ZONES[z].name };
        match map {
            OnMap::World => tr(self.es, "World map", "Mapa del mundo").to_string(),
            OnMap::Zone(z) => zone(z).to_string(),
            OnMap::NewZone(k) => match self.doc.new_zones.get(k) {
                Some(n) if !n.name.is_empty() => format!("{} ({})", n.name, zone(n.base)),
                Some(n) => format!("{} {} ({})", tr(self.es, "New zone", "Zona nueva"), k + 1, zone(n.base)),
                None => tr(self.es, "New zone", "Zona nueva").to_string(),
            },
        }
    }

    /// Screen position of a pixel of the shown map picture.
    pub(super) fn on_screen(&self, x: i32, y: i32) -> Vec2 {
        vec2(self.panel + (x as f32 - self.cam.x) * self.scale(), (y as f32 - self.cam.y) * self.scale())
    }

    /// The points drawn on the shown map: the game's own (where it draws them), the new ones
    /// of the world map or the game zone, on a new zone its own. (point, x, y, new)
    pub(super) fn map_points(&self) -> Vec<(u8, i32, i32, bool)> {
        if let Some(k) = self.new_zone_shown() {
            return self.doc.new_zones[k].points.iter().map(|p| (p.number, p.place.x as i32, p.place.y as i32 - FEET, true)).collect();
        }
        let mut v: Vec<(u8, i32, i32, bool)> = original_points::on(self.world_map).map(|s| self.spot_at(s)).collect();
        let added_here = self.doc.added_on() == self.world_map;
        for &p in &self.doc.routes.added {
            let pl = self.doc.routes.places[p as usize];
            match self.world_map {
                OnMap::World if added_here || added_points::index(p).is_none() => v.push((p, pl.on_world().0, pl.on_world().1, true)),
                OnMap::Zone(_) if added_here && added_points::index(p).is_some() => v.push((p, pl.x as i32, pl.y as i32 - FEET, true)),
                _ => {}
            }
        }
        v
    }

    /// Can new points be put on the shown map?
    fn takes_new_points(&self) -> bool {
        match self.world_map {
            OnMap::World | OnMap::NewZone(_) => true,
            OnMap::Zone(z) => z < added_points::ZONES,
        }
    }

    /// Where a game's point is drawn: where the game draws it, moved as far as its place was.
    fn spot_at(&self, s: &original_points::Spot) -> (u8, i32, i32, bool) {
        let (dx, dy) = self.doc.point_moved(self.world_map, s.point);
        (s.point, s.x + dx, s.y + dy, false)
    }

    /// How far a dragged point of the world map or a game zone has moved, as it will be placed.
    fn drag_offset(&self) -> Option<(i32, i32)> {
        let (_, from) = self.drag_from?;
        let d = self.world_pos(mouse_position().into()) - from;
        let snap = |v: f32| (v / 4.0).round() as i32 * 4;
        Some((snap(d.x), snap(d.y)))
    }

    /// The level point `p` of the shown map opens (`world::NO_LEVEL`: none).
    pub(super) fn level_of(&self, p: usize) -> u8 {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].point(p as u8).map_or(world::NO_LEVEL, |q| q.level),
            None => self.doc.points.get(p).copied().unwrap_or(world::NO_LEVEL),
        }
    }

    /// The ways out of point `p` of the shown map.
    fn ways_of(&self, p: usize) -> Option<&[Option<Way>; 4]> {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].point(p as u8).map(|q| &q.ways),
            None => self.doc.routes.ways.get(p),
        }
    }

    /// The point drawn under the mouse on the shown map.
    pub(super) fn point_at(&self, m: Vec2) -> Option<(u8, i32, i32, bool)> {
        let w = self.world_pos(m);
        self.map_points().into_iter().find(|s| (vec2(s.1 as f32, s.2 as f32) - w).length() <= 7.0)
    }

    /// Where the chosen point is drawn on the shown map, if it is.
    fn chosen_at(&self) -> Option<(i32, i32)> {
        self.map_points().into_iter().find(|s| s.0 as usize == self.point).map(|s| (s.1, s.2))
    }

    /// Is `p` a point of the world map (drawn there, the twin of a zone's entrance, or new)?
    pub(super) fn is_world_point(&self, p: u8) -> bool {
        (self.doc.routes.added.contains(&p) && (added_points::index(p).is_none() || self.doc.added_on() == OnMap::World)) || map_routes::ZONE_EXITS.iter().any(|z| z.1 == p) || original_points::SPOTS.iter().any(|s| s.point == p && s.map == OnMap::World)
    }

    pub(super) fn leads_name(&self, l: Leads) -> String {
        match l {
            Leads::World(p) => format!("{p:02X} ({})", tr(self.es, "world", "mundo")),
            Leads::Zone(i) => self.map_name(OnMap::Zone(doors::INTO[i].zone)),
            Leads::NewZone(k) => self.map_name(OnMap::NewZone(k)),
        }
    }

    /// The exit (index, where it leads) on point `p` of the shown map.
    pub(super) fn exit_on(&self, p: u8) -> Option<(usize, Leads)> {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].exits.iter().position(|e| e.0 == p).map(|i| (i, self.doc.new_zones[k].exits[i].1)),
            None => self.doc.doors.exit_on(p).map(|i| (i, self.doc.doors.exits[i].1)),
        }
    }

    /// Where world point `p`'s door leads, if anywhere.
    pub(super) fn door_of(&self, p: u8) -> Option<Leads> {
        self.doc.doors.door_of(p).map(Leads::Zone).or(self.doc.zone_door_of(p).map(Leads::NewZone))
    }

    /// The label of a point on the map: its number and, for a door or an exit, where it goes.
    fn point_tag(&self, p: u8) -> Option<String> {
        if self.new_zone_shown().is_none() {
            if self.world_map != OnMap::World && self.is_world_point(p) {
                // a world point drawn where Mario walks out of a zone
                return Some(format!("{p:02X} ({})", tr(self.es, "world", "mundo")));
            }
            if let Some(to) = self.door_of(p) {
                return Some(format!("{p:02X} > {}", self.leads_name(to)));
            }
        }
        let (_, leads) = self.exit_on(p)?;
        Some(format!("{p:02X} > {}", self.leads_name(leads)))
    }

    /// The exits of the shown zone that can be moved to the chosen point.
    fn exit_sites(&self) -> Vec<usize> {
        let p = self.point as u8;
        if self.doc.doors.into.is_empty() || self.exit_on(p).is_some() || self.chosen_at().is_none() {
            return Vec::new();
        }
        if let Some(k) = self.new_zone_shown() {
            return (0..self.doc.new_zones[k].exits.len()).collect();
        }
        let OnMap::Zone(z) = self.world_map else { return Vec::new() };
        if self.is_world_point(p) {
            return Vec::new();
        }
        (0..doors::EXITS.len()).filter(|&i| original_points::SPOTS.iter().any(|s| s.point == doors::EXITS[i].point && s.map == OnMap::Zone(z))).collect()
    }

    /// Where exit `i` of the shown zone is and leads.
    fn exit_at(&self, i: usize) -> (u8, Leads) {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].exits[i],
            None => self.doc.doors.exits[i],
        }
    }

    pub(super) fn set_exit(&mut self, i: usize, at: u8, leads: Leads) {
        match self.new_zone_shown() {
            Some(k) => self.doc.zone_set_exit(k, i, at, leads),
            None => self.doc.set_exit(i, at, leads),
        }
    }

    /// Move the `k`th exit of the shown zone to the chosen point.
    fn make_exit(&mut self, k: usize) {
        if let Some(&i) = self.exit_sites().get(k) {
            let leads = self.exit_at(i).1;
            self.set_exit(i, self.point as u8, leads);
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
        let target = match self.exit_on(p) {
            Some((_, leads)) => Some((tr(self.es, "Exit to", "Sale a"), self.leads_name(leads))),
            None if self.world_map == OnMap::World && self.is_world_point(p) => Some((
                tr(self.es, "Door", "Puerta"),
                match self.door_of(p) {
                    Some(to) => format!("{} {}", tr(self.es, "into", "entra en"), self.leads_name(to)),
                    None => tr(self.es, "none", "ninguna").to_string(),
                },
            )),
            None => None,
        };
        if let Some((name, value)) = target {
            draw_text(name, 10.0, y + 18.0, 18.0, col(3));
            draw_text(&value, 104.0, y + 18.0, 16.0, col(3));
            y += 26.0;
            if button(Rect::new(10.0, y, 276.0, 26.0), &with_keys(tr(self.es, "Choose on the map", "Elegir en el mapa"), Action::ChooseTarget), false, 15.0) {
                act = Some(Action::ChooseTarget);
            }
            y += 34.0;
        } else {
            for (k, i) in self.exit_sites().into_iter().enumerate().take(2) {
                let at = self.exit_at(i).0;
                let a = if k == 0 { Action::MakeExit1 } else { Action::MakeExit2 };
                let now = if at == doors::NONE { tr(self.es, "on no point", "en ningun punto").to_string() } else { format!("{} {at:02X}", tr(self.es, "now", "ahora")) };
                let label = format!("{} ({now})", tr(self.es, "Make it the exit", "Hacer salida aqui"));
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
        let maps = if self.picker.is_some() { self.picking_maps() } else { self.all_maps() };
        let i = maps.iter().position(|&m| m == self.world_map).unwrap_or(0) as i32;
        self.show_map(maps[(i + step).rem_euclid(maps.len() as i32) as usize]);
        self.choose_on_new_zone();
    }

    /// On a new zone, choose its entry point if the chosen point is not one of its own.
    fn choose_on_new_zone(&mut self) {
        if let Some(k) = self.new_zone_shown() {
            let z = &self.doc.new_zones[k];
            if z.point(self.point as u8).is_none() {
                self.point = z.entry as usize;
            }
        }
    }

    /// A new zone made from the map shown, then shown.
    pub fn new_zone(&mut self) {
        let from = self.world_map;
        self.status = match self.doc.add_zone(from) {
            Ok(k) => {
                self.clipboard = None;
                self.show_map(OnMap::NewZone(k));
                self.choose_on_new_zone();
                format!("{} {}", self.map_name(OnMap::NewZone(k)), tr(self.es, "made; it has no door yet (Points tab).", "creada; aun no tiene puerta (pestana Puntos)."))
            }
            Err(ZoneRefusal::TooManyZones(max)) => format!("{} {max}.", tr(self.es, "No more new zones: there can be", "No caben mas zonas nuevas: puede haber")),
            Err(_) => tr(self.es, "New zones are made from the Tree, Pumpkin, Macro, Space or Mario zone, or from a new zone.", "Las zonas nuevas se hacen de la Zona Arbol, Calabaza, Macro, Espacial o Mario, o de otra zona nueva.").into(),
        };
    }

    /// Delete the new zone shown; its base is shown.
    fn delete_zone(&mut self) {
        let Some(k) = self.new_zone_shown() else {
            self.status = tr(self.es, "Only new zones can be deleted; the game's are only edited.", "Solo se borran las zonas nuevas; las del juego solo se editan.").into();
            return;
        };
        let name = self.map_name(OnMap::NewZone(k));
        let base = self.doc.new_zones[k].base;
        self.doc.remove_zone(k);
        self.clipboard = None;
        self.show_map(OnMap::Zone(base));
        self.status = format!("{name} {}", tr(self.es, "deleted; doors and exits into it lead nowhere.", "borrada; las puertas y salidas a ella no llevan a ningun sitio."));
    }

    /// The Points tab's actions.
    pub(super) fn run_points(&mut self, a: Action) {
        let n = world::POINT_COUNT;
        let zone = self.new_zone_shown();
        if self.picker.is_some() {
            return;
        }
        match a {
            Action::NewZone => self.new_zone(),
            Action::DeleteZone => self.delete_zone(),
            Action::PrevPoint | Action::NextPoint if zone.is_some() => {
                let list: Vec<usize> = self.map_points().iter().map(|s| s.0 as usize).collect();
                if !list.is_empty() {
                    let i = list.iter().position(|&p| p == self.point).unwrap_or(0) as i32;
                    let step = if a == Action::NextPoint { 1 } else { -1 };
                    self.point = list[(i + step).rem_euclid(list.len() as i32) as usize];
                }
            }
            Action::PrevPoint => self.select_original_point((self.point + n - 1) % n),
            Action::NextPoint => self.select_original_point((self.point + 1) % n),
            Action::PrevOpens | Action::NextOpens => {
                let step = if a == Action::NextOpens { 1 } else { -1 };
                match zone {
                    Some(k) => self.doc.zone_step_level(k, self.point as u8, step),
                    None => self.doc.step_point_level(self.point, step),
                }
            }
            Action::ChooseTarget => self.start_picking(),
            Action::MakeExit1 => self.make_exit(0),
            Action::MakeExit2 => self.make_exit(1),
            Action::MakeEntry => match (zone, self.world_map) {
                (Some(k), _) => self.doc.zone_set_entry(k, self.point as u8),
                (None, OnMap::Zone(z)) if self.can_be_game_entry() => self.doc.set_game_entry(z, self.point as u8),
                _ => {}
            },
            Action::DeletePathRight | Action::DeletePathLeft | Action::DeletePathUp | Action::DeletePathDown => {
                let d = [Action::DeletePathRight, Action::DeletePathLeft, Action::DeletePathUp, Action::DeletePathDown].iter().position(|&x| x == a).unwrap_or(0);
                match zone {
                    Some(k) => self.doc.zone_set_way(k, self.point as u8, d, None),
                    None if self.point < map_routes::ALL => self.doc.set_way(self.point, d, None),
                    None => {}
                }
            }
            Action::ShutRight | Action::ShutLeft | Action::ShutUp | Action::ShutDown => {
                let d = Self::SHUT_ACTIONS.iter().position(|&x| x == a).unwrap_or(0);
                if self.doc.can_shut(zone, self.point as u8) {
                    self.doc.toggle_shut(zone, self.point as u8, d);
                } else {
                    self.status = tr(self.es, "Only new points can shut their paths until their level is cleared.", "Solo los puntos nuevos pueden cerrar sus caminos hasta superar su nivel.").into();
                }
            }
            Action::NewPath => {
                let drawing = self.route_draw.is_some();
                let can_draw = self.point < map_routes::ALL && self.chosen_at().is_some();
                self.placing_point = false;
                self.route_draw = if drawing || !can_draw { None } else { Some(Vec::new()) };
                if !can_draw {
                    self.status = tr(self.es, "Choose a point on the map shown first.", "Elige antes un punto del mapa que se ve.").into();
                }
            }
            Action::NewPoint if zone.is_some_and(|k| self.doc.new_zones[k].points.len() >= self.doc.new_zones[k].room()) && !self.placing_point => {
                self.status = self.zone_full_note(zone.unwrap_or(0));
            }
            Action::NewPoint if self.takes_new_points() => {
                self.route_draw = None;
                self.placing_point = !self.placing_point;
            }
            Action::DeletePoint => match zone {
                Some(k) => {
                    if !self.doc.zone_remove_point(k, self.point as u8) {
                        self.status = tr(self.es, "A new zone keeps at least one point.", "Una zona nueva conserva al menos un punto.").into();
                    }
                    self.choose_on_new_zone();
                }
                None if self.doc.routes.added.contains(&(self.point as u8)) => {
                    self.doc.remove_point(self.point as u8);
                }
                None => {}
            },
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
            self.show_map(if added_points::index(p as u8).is_some() { self.doc.added_on() } else { OnMap::World });
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

    /// Why new zone `k` takes no more points.
    fn zone_full_note(&self, k: usize) -> String {
        let z = &self.doc.new_zones[k];
        format!(
            "{} {} {}",
            tr(self.es, "This zone already has", "Esta zona ya tiene"),
            z.room(),
            tr(self.es, "points, as many as a zone can have (the game picks the map by point number). Delete one (Del) to put it somewhere else.", "puntos, todos los que puede tener una zona (el juego elige el mapa por el numero del punto). Borra uno (Supr) para ponerlo en otro sitio."),
        )
    }

    /// Put a new point where the mouse is (the world map, or the new zone shown).
    fn place_point(&mut self, m: Vec2) {
        self.placing_point = false;
        if let Some(k) = self.new_zone_shown() {
            let w = self.world_pos(m);
            self.status = match self.doc.zone_add_point(k, w.x.round() as i32, w.y.round() as i32 + FEET) {
                Ok(p) => {
                    self.point = p as usize;
                    format!("{} {p:02X}", tr(self.es, "New point", "Punto nuevo"))
                }
                Err(ZoneRefusal::NoNumberLeft(_)) => self.zone_full_note(k),
                Err(_) => String::new(),
            };
            return;
        }
        // world points sit 2 pixels below the 4-pixel grid; a zone's places are Mario's feet
        let (x, y) = match self.world_map {
            OnMap::World => self.snapped(m, (0, 2)),
            _ => {
                let w = self.world_pos(m);
                (((w.x / 4.0).round() * 4.0) as i32, ((w.y / 4.0).round() * 4.0) as i32 + FEET)
            }
        };
        self.status = match self.doc.add_point(self.world_map, x, y) {
            Some(p) => {
                self.point = p as usize;
                format!("{} {p:02X}", tr(self.es, "New point", "Punto nuevo"))
            }
            None => tr(self.es, "This map has as many points as it can have.", "Este mapa ya tiene todos los puntos que puede tener.").into(),
        };
    }

    /// Clicks on the map in the Points tab: choose a point, put a new one, draw a path, or
    /// drag a new zone's point somewhere else.
    fn points_clicks(&mut self, m: Vec2) {
        let clicked = is_mouse_button_pressed(MouseButton::Left) && m.x >= self.panel && m.y < screen_height() - STATUS_H;
        if self.placing_point {
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                self.placing_point = false;
            } else if clicked {
                self.place_point(m);
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
                Some((p, x, y, _)) if p as usize != self.point && (p as usize) < map_routes::ALL => {
                    let mut rel: Vec<(i32, i32)> = self.route_draw.take().unwrap_or_default().iter().map(|c| (c.0 - start.0, c.1 - start.1)).collect();
                    rel.push((x - start.0, y - start.1));
                    let made = match self.new_zone_shown() {
                        Some(k) => self.doc.zone_draw_path(k, self.point as u8, p, &rel),
                        None => self.doc.draw_path(self.point, p, &rel),
                    };
                    self.status = match made {
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
        if let Some(p) = self.drag_point {
            if !is_mouse_button_down(MouseButton::Left) {
                let offset = self.drag_offset();
                self.drag_point = None;
                let from = self.drag_from.take();
                match (self.new_zone_shown(), from, offset) {
                    (Some(k), _, _) => {
                        let w = self.world_pos(m);
                        self.doc.zone_move_point(k, p, w.x.round() as i32, w.y.round() as i32 + FEET);
                    }
                    (None, Some((place, _)), Some((dx, dy))) if (dx, dy) != (0, 0) => {
                        self.doc.points_snapshot();
                        self.doc.move_point(self.world_map, p, place, dx, dy);
                    }
                    _ => {}
                }
            }
            return;
        }
        if clicked {
            if let Some(s) = self.point_at(m) {
                self.point = s.0 as usize;
                let movable = self.new_zone_shown().is_some() || !(self.world_map != OnMap::World && self.is_world_point(s.0));
                if movable && (s.0 as usize) < map_routes::ALL {
                    self.drag_point = Some(s.0);
                    self.drag_from = self.doc.routes.places.get(s.0 as usize).map(|&pl| (pl, self.world_pos(m)));
                }
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
        // a new zone gone (undone project, deleted): show the world map
        if matches!(self.world_map, OnMap::NewZone(_)) && self.new_zone_shown().is_none() {
            self.show_map(OnMap::World);
        }
        let m: Vec2 = mouse_position().into();
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let (space, _, _) = self.navigate(m, ctrl, self.map_px());
        if self.pan.is_some() || space {
            return;
        }
        if self.picker.is_some() && self.world_tab != TAB_POINTS {
            self.cancel_picking();
        }
        if self.world_tab == TAB_POINTS {
            if self.picker.is_some() {
                self.picker_clicks(m);
            } else {
                self.points_clicks(m);
            }
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
            let x0 = (self.cam.x / 8.0).floor().max(0.0) as i32;
            let y0 = (self.cam.y / 8.0).floor().max(0.0) as i32;
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
                self.draw_points();
                self.draw_picker();
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
            _ if self.picker.is_some() => self.picker_panel(y).or(act),
            _ => self.points_panel(y).or(act),
        };
        self.status_line(cw, ch);
        if let Some(a) = act {
            self.run(a);
        }
    }

    /// Restore the map shown: picture and points (`EditorDocument::restore_map`).
    pub(super) fn restore_map(&mut self) {
        let map = self.world_map;
        let name = self.map_name(map);
        self.clipboard = None;
        self.route_draw = None;
        self.placing_point = false;
        if !self.doc.restore_map(map) {
            return;
        }
        self.choose_on_new_zone();
        let how = match map {
            OnMap::NewZone(k) => format!("{} {} {}", tr(self.es, "restored as the", "restaurada como la"), self.map_name(OnMap::Zone(self.doc.new_zones[k].base)), tr(self.es, "is in the game", "esta en el juego")),
            _ => tr(self.es, "restored as it is in the game", "restaurado como esta en el juego").to_string(),
        };
        self.status = format!("{name} {how}: {}", tr(self.es, "picture and points (save to keep it; undo in each tab brings them back).", "dibujo y puntos (guarda para conservarlo; deshacer en cada pestana lo recupera)."));
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

    /// Title, map choice, new zones, tabs, tools, undo and save. Returns the action chosen and
    /// where the panel continues.
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
        // which map: the world map, a zone map or a new zone
        let mut name = self.map_name(self.world_map);
        if self.doc.mods.map(self.world_map) {
            name.push_str(" (mod)");
        }
        let d = measure_text(&name, None, 18, 1.0);
        draw_text(&name, 148.0 - d.width / 2.0, y + 15.0, 18.0, col(3));
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
        let mut b = |r: Rect, name: &str, a: Action, on: bool| {
            if button(r, &with_keys(name, a), on, 15.0) {
                act = Some(a);
            }
        };
        if self.world_map != OnMap::World {
            b(Rect::new(col2(0), y, w2, 26.0), tr(es, "New zone", "Zona nueva"), Action::NewZone, false);
            if self.new_zone_shown().is_some() {
                b(Rect::new(col2(1), y, w2, 26.0), tr(es, "Delete zone", "Borrar zona"), Action::DeleteZone, false);
            }
            y += 30.0;
        }
        if let Some(k) = self.new_zone_shown() {
            self.name_field(Rect::new(10.0, y, 276.0, 24.0), Named::Zone(k));
            y += 28.0;
        }
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
            b(Rect::new(col3(1), y, 276.0 - w3 - 4.0, 26.0), tr(es, "Restore", "Restaurar"), Action::Restore, false);
        } else {
            b(Rect::new(10.0, y, 276.0, 26.0), tr(es, "Restore", "Restaurar"), Action::Restore, false);
        }
        y += 30.0;
        b(Rect::new(10.0, y, 276.0, 28.0), tr(es, "Save", "Guardar"), Action::Save, false);
        y += 36.0;
        (act, y)
    }

    /// A point's circle with the level it opens and its tag; the chosen one ringed.
    fn draw_point(&self, p: u8, (x, y): (i32, i32), fill: Color, tag: &str) {
        let r = (4.0 * self.scale()).max(6.0);
        let c = self.on_screen(x, y);
        let level = self.level_of(p as usize);
        if (level as usize) < self.doc.levels.len() {
            let fs = (r * 1.3).clamp(13.0, 22.0);
            draw_circle(c.x, c.y, r + 2.0, BLACK);
            draw_circle(c.x, c.y, r, fill);
            let t = format!("{level:02}");
            let d = measure_text(&t, None, fs as u16, 1.0);
            draw_text(&t, c.x - d.width / 2.0, c.y + d.offset_y / 2.0, fs, WHITE);
        } else {
            draw_circle(c.x, c.y, r * 0.6 + 1.5, BLACK);
            draw_circle(c.x, c.y, r * 0.6, Color::new(0.8, 0.8, 0.8, 1.0));
        }
        draw_text(tag, c.x + r + 3.0, c.y - r * 0.4 + 1.0, 15.0, BLACK);
        draw_text(tag, c.x + r + 2.0, c.y - r * 0.4, 15.0, WHITE);
        if p as usize == self.point {
            draw_circle_lines(c.x, c.y, r + 5.0, 3.0, WHITE);
        }
    }

    /// The points on the shown map: the level each opens, the paths Mario walks out of each
    /// (the chosen point's highlighted), the chosen one ringed.
    fn draw_points(&self) {
        let r = (4.0 * self.scale()).max(6.0);
        let pts = self.map_points();
        let zone = self.new_zone_shown();
        for &(p, x, y, _) in &pts {
            let Some(ways) = self.ways_of(p as usize) else { continue };
            if zone.is_none() && self.world_map != OnMap::World && self.is_world_point(p) {
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
        if self.placing_point || self.drag_point.is_some() {
            let dragged = self.drag_point.and_then(|p| Some((pts.iter().find(|s| s.0 == p)?, self.drag_offset()?)));
            let (x, y) = if zone.is_some() {
                let w = self.world_pos(mouse_position().into());
                (w.x as i32, w.y as i32)
            } else if let Some((s, (dx, dy))) = dragged {
                (s.1 + dx, s.2 + dy)
            } else {
                self.snapped(mouse_position().into(), (0, 2))
            };
            let c = self.on_screen(x, y);
            draw_circle_lines(c.x, c.y, r, 2.0, WHITE);
        }
        if let Some(k) = zone {
            let entry = self.doc.new_zones[k].entry;
            for &(p, x, y, _) in &pts {
                let mut tag = self.point_tag(p).unwrap_or(format!("{p:02X}"));
                if p == entry {
                    tag = format!("{tag} {}", tr(self.es, "(entry)", "(entrada)"));
                }
                self.draw_point(p, (x, y), Color::new(0.95, 0.45, 0.1, 1.0), &tag);
            }
            return;
        }
        for (i, sp) in original_points::SPOTS.iter().enumerate() {
            if sp.map != self.world_map {
                continue;
            }
            let mut tag = match (self.point_tag(sp.point), original_points::door(i)) {
                (Some(t), _) => t,
                (None, Some(to)) if self.doc.doors.into.is_empty() => format!("{:02X} > {}", sp.point, self.map_name(to.map)),
                _ => format!("{:02X}", sp.point),
            };
            if self.game_entry_shown() == Some(sp.point) {
                tag = format!("{tag} {}", tr(self.es, "(entry)", "(entrada)"));
            }
            let (_, x, y, _) = self.spot_at(sp);
            self.draw_point(sp.point, (x, y), Color::new(0.95, 0.45, 0.1, 1.0), &tag);
        }
        for &(p, x, y, new) in &pts {
            if new {
                let tag = format!("{} {}", self.point_tag(p).unwrap_or(format!("{p:02X}")), tr(self.es, "(new)", "(nuevo)"));
                self.draw_point(p, (x, y), Color::new(0.2, 0.7, 0.3, 1.0), &tag);
            }
        }
    }

    const SHUT_ACTIONS: [Action; 4] = [Action::ShutRight, Action::ShutLeft, Action::ShutUp, Action::ShutDown];

/// The chosen point: the level it opens, where it leads, its paths; the points of the map.
    fn points_panel(&mut self, mut y: f32) -> Option<Action> {
        let zone = self.new_zone_shown();
        let (used, most) = self.doc.point_count(self.world_map);
        draw_text(&format!("{} {used} / {most}", tr(self.es, "Points:", "Puntos:")), 10.0, y + 12.0, 16.0, col(3));
        y += 20.0;
        let mut act = self.stepper(y, tr(self.es, "Point", "Punto"), &format!("{:02X}", self.point), 130.0, (Action::PrevPoint, Action::NextPoint));
        let new = self.doc.routes.added.contains(&(self.point as u8));
        let on_zone = zone.is_some_and(|k| self.doc.new_zones[k].point(self.point as u8).is_some());
        let drawn = if on_zone {
            Some(self.map_name(self.world_map))
        } else if zone.is_some() {
            None
        } else if new {
            Some(tr(self.es, "new point", "punto nuevo").to_string())
        } else {
            original_points::SPOTS.iter().find(|s| s.point as usize == self.point).map(|s| self.map_name(s.map))
        };
        draw_text(&drawn.unwrap_or_else(|| tr(self.es, "(not drawn)", "(no se dibuja)").to_string()), 162.0, y + 18.0, 15.0, col(2));
        y += 34.0;
        let cur = self.level_of(self.point);
        let level = (cur as usize) < self.doc.levels.len();
        let label = match cur as usize {
            l if level && l >= cart_model::levels::LEVELS => match self.doc.level_name(l) {
                "" => format!("{} {l:02} ({} {:02})", tr(self.es, "level", "nivel"), tr(self.es, "new, as", "nuevo, como"), self.doc.base_of(l)),
                name => format!("{l:02} {name} ({} {:02})", tr(self.es, "as", "como"), self.doc.base_of(l)),
            },
            l if level => format!("{} {l:02}", tr(self.es, "level", "nivel")),
            _ if cur == world::NO_LEVEL => tr(self.es, "no level", "sin nivel").to_string(),
            _ => format!("{} ${cur:02X}", tr(self.es, "special", "especial")),
        };
        act = act.or(self.stepper(y, tr(self.es, "Opens", "Abre"), &label, 210.0, (Action::PrevOpens, Action::NextOpens)));
        y += 34.0;
        let (ny, door) = self.door_rows(y);
        y = ny;
        act = act.or(door);
        // the ways out of the point
        draw_text(tr(self.es, "Paths (pad > point); button: delete", "Caminos (cruceta > punto); boton: borrar"), 10.0, y + 12.0, 15.0, col(2));
        y += 18.0;
        if let Some(ways) = self.ways_of(self.point).cloned() {
            for (d, way) in ways.iter().enumerate() {
                let r = Rect::new(10.0 + (d % 2) as f32 * 138.0, y + (d / 2) as f32 * 26.0, 134.0, 24.0);
                match way.as_ref().map(|w| w.to) {
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
            if self.doc.can_shut(zone, self.point as u8) {
                draw_text(tr(self.es, "Shut until its level is cleared (Ctrl+arrows):", "Cerrado hasta superar su nivel (Ctrl+flechas):"), 10.0, y + 12.0, 15.0, col(2));
                y += 18.0;
                let shut = self.doc.shut_of(zone, self.point as u8);
                for (d, &a) in Self::SHUT_ACTIONS.iter().enumerate() {
                    let on = shut & added_points::DIR_BITS[d] != 0;
                    if button(Rect::new(10.0 + d as f32 * 69.0, y, 65.0, 24.0), &DIRS[d].to_string(), on, 15.0) {
                        act = Some(a);
                    }
                }
                y += 30.0;
            }
        }
        if button(Rect::new(10.0, y, 134.0, 26.0), &with_keys(tr(self.es, "New path", "Nuevo camino"), Action::NewPath), self.route_draw.is_some(), 15.0) {
            act = Some(Action::NewPath);
        }
        if self.takes_new_points() && button(Rect::new(148.0, y, 138.0, 26.0), &with_keys(tr(self.es, "New point", "Nuevo punto"), Action::NewPoint), self.placing_point, 15.0) {
            act = Some(Action::NewPoint);
        }
        y += 30.0;
        if new || on_zone {
            if button(Rect::new(10.0, y, 134.0, 26.0), &with_keys(tr(self.es, "Delete point", "Borrar punto"), Action::DeletePoint), false, 15.0) {
                act = Some(Action::DeletePoint);
            }
            if let Some(k) = zone.filter(|_| on_zone) {
                let is_entry = self.doc.new_zones[k].entry as usize == self.point;
                if button(Rect::new(148.0, y, 138.0, 26.0), &with_keys(tr(self.es, "Entry", "Entrada"), Action::MakeEntry), is_entry, 15.0) {
                    act = Some(Action::MakeEntry);
                }
            }
            y += 30.0;
        } else if self.can_be_game_entry() {
            let is_entry = self.game_entry_shown() == Some(self.point as u8);
            if button(Rect::new(10.0, y, 276.0, 26.0), &with_keys(tr(self.es, "Entry (doors and exits into the zone)", "Entrada (puertas y salidas a la zona)"), Action::MakeEntry), is_entry, 15.0) {
                act = Some(Action::MakeEntry);
            }
            y += 30.0;
        }
        y += 4.0;
        let count = zone.map(|k| format!(" ({} / {})", self.doc.new_zones[k].points.len(), self.doc.new_zones[k].room())).unwrap_or_default();
        let title = format!("{} {}{count}:", tr(self.es, "Points on", "Puntos en"), self.map_name(self.world_map));
        draw_text(&title, 10.0, y + 12.0, 15.0, col(2));
        y += 18.0;
        let list: Vec<usize> = self.map_points().iter().map(|s| s.0 as usize).collect();
        for (k, &p) in list.iter().enumerate() {
            let r = Rect::new(10.0 + (k % 3) as f32 * 92.0, y + (k / 3) as f32 * 24.0, 88.0, 22.0);
            let l = self.level_of(p);
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
            tr(self.es, "Click on the map where the new point goes. Esc: cancel.", "Haz clic en el mapa donde va el punto nuevo. Esc: cancelar.")
        } else if zone.is_some() {
            tr(
                self.es,
                "A new zone plays with its base zone's graphics and music. Click a point to choose it, drag it to move it (its paths become straight); New point adds one (up to 20 in the Tree Zone's, 21 in the others). Entry: Mario starts there. Exit to: where its exits lead. Its door is chosen on a world map point. Plays only in this port.",
                "Una zona nueva se juega con los graficos y la musica de su zona base. Haz clic en un punto para elegirlo y arrastralo para moverlo (sus caminos quedan rectos); Nuevo punto anade uno (hasta 20 en las de la Zona Arbol, 21 en las demas). Entrada: Mario empieza ahi. Sale a: adonde llevan sus salidas. Su puerta se elige en un punto del mundo. Solo se juega en este port.",
            )
        } else {
            tr(
                self.es,
                "Click a point on the map to choose it; with - + choose the level it opens. Door: a world point can enter a zone or a new zone (A or START on it). Exit to: a zone's exit leads to a world point or straight into another zone; Choose on the map (U) picks where by clicking. Make it the exit moves it to the chosen zone point. Paths are drawn in yellow; New path draws one, New point puts one on the world map (6 numbers free). Play (F5) saves and tries it.",
                "Haz clic en un punto del mapa para elegirlo; con - + elige el nivel que abre. Puerta: un punto del mundo puede entrar en una zona o en una zona nueva (A o START sobre el). Sale a: la salida de una zona lleva a un punto del mundo o directamente a otra zona; Elegir en el mapa (U) lo elige con un clic. Hacer salida aqui la mueve al punto elegido de la zona. Los caminos se ven en amarillo; Nuevo camino dibuja uno, Nuevo punto pone uno en el mundo (6 numeros libres). Jugar (F5) guarda y lo prueba.",
            )
        };
        for (i, line) in wrap(note, 36).iter().enumerate() {
            draw_text(line, 10.0, yy + i as f32 * 16.0, 15.0, col(2));
        }
        act
    }
}
