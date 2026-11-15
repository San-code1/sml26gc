//! Choosing where a door or an exit leads by looking: the maps it can lead to are shown one at
//! a time (tabs over the map), and a click picks a world map point or a zone (a door or an
//! exit into a zone enters it on the point clicked, which becomes that zone's entry point for
//! every door and exit into it). Esc or right click cancels.
use super::shortcuts::{with_keys, Action};
use super::widgets::*;
use super::Editor;
use cart_model::doors::{self, Leads};
use cart_model::new_zones;
use cart_model::original_points::OnMap;
use macroquad::prelude::*;

/// What is being chosen: where world point `p`'s door leads, or where exit `i` of the zone
/// that was shown leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Picking {
    Door(u8),
    Exit(usize),
}

pub(super) struct Picker {
    what: Picking,
    /// The map, point and view to go back to.
    back: OnMap,
    point: usize,
    cam: Vec2,
}

const TAB_H: f32 = 24.0;

impl Editor {
    /// Choose on the maps where the chosen point's door or exit leads (for the command line).
    pub fn choose_target(&mut self) {
        self.start_picking();
    }

    /// The maps the target can be on: the world map (exits only), the zones a door enters, the
    /// new zones.
    fn picker_maps(&self, what: Picking) -> Vec<OnMap> {
        let world = matches!(what, Picking::Exit(_)).then_some(OnMap::World);
        world.into_iter().chain(doors::INTO.iter().map(|d| OnMap::Zone(d.zone))).chain((0..self.doc.new_zones.len()).map(OnMap::NewZone)).collect()
    }

    /// Where a target is shown.
    fn map_of(&self, l: Leads) -> OnMap {
        match l {
            Leads::World(_) => OnMap::World,
            Leads::Zone(i) => OnMap::Zone(doors::INTO[i].zone),
            Leads::NewZone(k) => OnMap::NewZone(k),
        }
    }

    /// Start choosing where the chosen point's door or exit leads.
    pub(super) fn start_picking(&mut self) {
        if self.doc.doors.into.is_empty() {
            return;
        }
        let p = self.point as u8;
        let (what, now) = match self.exit_on(p) {
            Some((i, leads)) => (Picking::Exit(i), Some(leads)),
            None if self.world_map == OnMap::World && self.is_world_point(p) => (Picking::Door(p), self.door_of(p)),
            None => return,
        };
        self.route_draw = None;
        self.placing_point = false;
        self.picker = Some(Picker { what, back: self.world_map, point: self.point, cam: self.cam });
        let maps = self.picker_maps(what);
        let first = now.map(|l| self.map_of(l)).filter(|m| maps.contains(m)).unwrap_or(maps[0]);
        self.show_picking(first);
        self.status = tr(self.es, "Click where it leads: a world map point, or the point of a zone Mario enters on (it becomes the zone's entry). Tabs: the maps. Esc: cancel.", "Haz clic adonde lleva: un punto del mapa del mundo, o el punto de una zona por el que entra Mario (pasa a ser la entrada de la zona). Pestanas: los mapas. Esc: cancelar.").into();
    }

    /// Back to the map that was shown; `to`: the target chosen.
    fn finish_picking(&mut self, to: Option<Option<(Leads, Option<u8>)>>) {
        let Some(pk) = self.picker.take() else { return };
        self.show_map(pk.back);
        (self.point, self.cam) = (pk.point, pk.cam);
        let Some(to) = to else {
            self.status = tr(self.es, "Nothing changed.", "No se ha cambiado nada.").into();
            return;
        };
        match to {
            Some((Leads::Zone(i), Some(p))) => self.doc.set_game_entry(doors::INTO[i].zone, p),
            Some((Leads::NewZone(k), Some(p))) => self.doc.zone_set_entry(k, p),
            _ => {}
        }
        match (pk.what, to.map(|t| t.0)) {
            (Picking::Exit(i), Some(l)) => self.set_exit(i, pk.point as u8, l),
            (Picking::Door(p), None) => {
                self.doc.set_door(p, None);
                self.doc.set_zone_door(p, None);
            }
            (Picking::Door(p), Some(Leads::Zone(z))) => self.doc.set_door(p, Some(z)),
            (Picking::Door(p), Some(Leads::NewZone(k))) => self.doc.set_zone_door(p, Some(k)),
            _ => return,
        }
        let name = to.map_or(tr(self.es, "nowhere", "ningun sitio").to_string(), |t| self.target_name(t));
        self.status = format!("{:02X} {} {name}", pk.point, tr(self.es, "now leads to", "ahora lleva a"));
    }

    /// The target on the shown map under the mouse: a world point, or a point of the zone
    /// shown to enter it on.
    fn target_at(&self, m: Vec2) -> Option<(Leads, Option<u8>)> {
        let p = self.point_at(m)?.0;
        match self.world_map {
            OnMap::World => self.is_world_point(p).then_some((Leads::World(p), None)),
            OnMap::Zone(z) => {
                let i = doors::INTO.iter().position(|d| d.zone == z)?;
                new_zones::base_of(z).is_some_and(|b| b.has(p)).then_some((Leads::Zone(i), Some(p)))
            }
            OnMap::NewZone(k) => self.doc.new_zones.get(k)?.point(p).map(|_| (Leads::NewZone(k), Some(p))),
        }
    }

    fn target_name(&self, (l, entry): (Leads, Option<u8>)) -> String {
        match entry {
            Some(p) => format!("{}, {} {p:02X}", self.leads_name(l), tr(self.es, "entering on", "entrando por")),
            None => self.leads_name(l),
        }
    }

    /// Clicks while choosing (the tabs are buttons, drawn with the map).
    pub(super) fn picker_clicks(&mut self, m: Vec2) {
        if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
            self.finish_picking(None);
            return;
        }
        let tabs = self.picker_tabs_h();
        if is_mouse_button_pressed(MouseButton::Left) && m.y > tabs && m.y < screen_height() - STATUS_H {
            if let Some(t) = self.target_at(m) {
                self.finish_picking(Some(Some(t)));
            }
        }
    }

    fn tab_rects(&self) -> Vec<(OnMap, Rect, String)> {
        let Some(pk) = &self.picker else { return Vec::new() };
        let (mut x, mut y) = (self.panel + 6.0, 6.0);
        let mut v = Vec::new();
        for map in self.picker_maps(pk.what) {
            let name = self.map_name(map);
            let w = measure_text(&name, None, 15, 1.0).width + 16.0;
            if x + w > screen_width() - 6.0 {
                (x, y) = (self.panel + 6.0, y + TAB_H + 4.0);
            }
            v.push((map, Rect::new(x, y, w, TAB_H), name));
            x += w + 4.0;
        }
        v
    }

    pub(super) fn picker_tabs_h(&self) -> f32 {
        self.tab_rects().last().map_or(0.0, |t| t.1.y + TAB_H + 6.0)
    }

    /// The tabs over the map and, on a zone, its entry point marked.
    pub(super) fn draw_picker(&mut self) {
        if self.picker.is_none() {
            return;
        }
        let r = (4.0 * self.scale()).max(6.0);
        let entry = match self.world_map {
            OnMap::Zone(z) => self.doc.game_entry(z),
            OnMap::NewZone(k) => self.doc.new_zones.get(k).map(|z| z.entry),
            OnMap::World => None,
        };
        if let Some((_, x, y, _)) = entry.and_then(|e| self.map_points().into_iter().find(|s| s.0 == e)) {
            let c = self.on_screen(x, y);
            draw_circle_lines(c.x, c.y, r + 8.0, 3.0, Color::new(1.0, 0.85, 0.1, 1.0));
            let t = tr(self.es, "enters here", "entra aqui");
            draw_text(t, c.x + r + 10.0, c.y + 5.0, 18.0, BLACK);
            draw_text(t, c.x + r + 9.0, c.y + 4.0, 18.0, Color::new(1.0, 0.95, 0.5, 1.0));
        }
        if let Some(l) = self.target_at(mouse_position().into()) {
            let t = self.target_name(l);
            let m: Vec2 = mouse_position().into();
            draw_text(&t, m.x + 15.0, m.y + 21.0, 18.0, BLACK);
            draw_text(&t, m.x + 14.0, m.y + 20.0, 18.0, WHITE);
        }
        let tabs = self.tab_rects();
        if let Some(t) = tabs.last() {
            draw_rectangle(self.panel, 0.0, screen_width() - self.panel, t.1.y + TAB_H + 6.0, Color::new(0.0, 0.0, 0.0, 0.55));
        }
        for (map, rect, name) in tabs {
            if button(rect, &name, map == self.world_map, 15.0) {
                self.show_picking(map);
            }
        }
    }

    /// Show `map` while choosing, below the tabs.
    fn show_picking(&mut self, map: OnMap) {
        self.show_map(map);
        self.cam = vec2(0.0, -self.picker_tabs_h() / self.scale());
    }

    /// The maps the target being chosen can be on.
    pub(super) fn picking_maps(&self) -> Vec<OnMap> {
        self.picker.as_ref().map_or_else(Vec::new, |pk| self.picker_maps(pk.what))
    }

    pub(super) fn cancel_picking(&mut self) {
        self.finish_picking(None);
    }

    /// The panel while choosing: what is being chosen, no door, cancel.
    pub(super) fn picker_panel(&mut self, mut y: f32) -> Option<Action> {
        let Some(pk) = &self.picker else { return None };
        let (what, from) = (pk.what, pk.point);
        let title = match what {
            Picking::Door(_) => format!("{} {from:02X} {}", tr(self.es, "Door of", "Puerta de"), tr(self.es, "leads into:", "entra en:")),
            Picking::Exit(_) => format!("{} {from:02X} {}", tr(self.es, "Exit on", "Salida en"), tr(self.es, "leads to:", "lleva a:")),
        };
        draw_text(&title, 10.0, y + 16.0, 18.0, col(3));
        y += 28.0;
        let note = match what {
            Picking::Door(_) => tr(self.es, "Choose a zone with the tabs over the map and click the point Mario enters on: it becomes the zone's entry, for every door and exit into it.", "Elige una zona con las pestanas sobre el mapa y haz clic en el punto por el que entra Mario: pasa a ser la entrada de la zona, para todas las puertas y salidas a ella."),
            Picking::Exit(_) => tr(self.es, "World map: click the point Mario comes out on. A zone: click the point Mario enters on (it becomes the zone's entry).", "Mapa del mundo: haz clic en el punto donde sale Mario. Una zona: haz clic en el punto por el que entra Mario (pasa a ser la entrada de la zona)."),
        };
        for line in wrap(note, 36) {
            draw_text(&line, 10.0, y + 12.0, 15.0, col(2));
            y += 16.0;
        }
        y += 8.0;
        if matches!(what, Picking::Door(_)) && button(Rect::new(10.0, y, 276.0, 26.0), tr(self.es, "No door", "Sin puerta"), false, 15.0) {
            self.finish_picking(Some(None));
            return None;
        }
        y += 30.0;
        if button(Rect::new(10.0, y, 276.0, 26.0), &with_keys(tr(self.es, "Cancel", "Cancelar"), Action::Escape), false, 15.0) {
            self.finish_picking(None);
        }
        None
    }
}
