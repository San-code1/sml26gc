//! The object catalogue (F3): the game's objects once each (`object_catalogue`), with the
//! editor's names (`object_names`), filtered by category and by "usable here". Choosing one
//! the shown level draws right picks its type; an enemy of another level of the same object
//! catalogue can be added to the level's own enemy set (`enemy_sets`) when its tiles fit.
use super::level_panel::fit;
use super::shortcuts::{keys, Action};
use super::widgets::*;
use super::Editor;
use crate::domain::document::EnemySet;
use crate::domain::object_catalogue::CatalogueObject;
use crate::domain::object_names::{self, Category, CATEGORIES};
use cart_model::enemy_sets::{self, Enemy};
use macroquad::prelude::*;

const PAD: f32 = 10.0;
const ROW: f32 = 26.0;
const CELL: (f32, f32) = (150.0, 74.0);
/// The first level of the second object catalogue.
const SECOND: usize = 10;

/// The catalogue as shown: its objects, the filters, the one chosen, the view's scroll.
pub struct CatalogueView {
    objects: Vec<CatalogueObject>,
    filter: Option<Category>,
    usable_only: bool,
    chosen: Option<usize>,
    scroll: f32,
}

/// Whether an enemy of another level can join the shown level's enemy set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fit {
    Yes,
    /// Its tiles are those of this enemy of the set.
    Clash(Enemy),
    /// The set has as many enemies as it can.
    Full,
    /// Its tiles, or the set's, are still being captured.
    Pending,
}

/// A category as the palette's filter says it.
pub fn category_plural(es: bool, c: Option<Category>) -> &'static str {
    match c {
        None => tr(es, "All objects", "Todos los objetos"),
        Some(Category::PowerUp) => tr(es, "Power-ups", "Power-ups"),
        Some(Category::Enemy) => tr(es, "Enemies", "Enemigos"),
        Some(Category::Trap) => tr(es, "Traps", "Trampas"),
        Some(Category::Platform) => tr(es, "Platforms", "Plataformas"),
        Some(Category::Goal) => tr(es, "Goals", "Metas"),
        Some(Category::Marker) => tr(es, "Invisible markers", "Marcadores invisibles"),
        Some(Category::Scripted) => tr(es, "Boss and scripted", "Jefes y guiones"),
    }
}

pub fn category_name(es: bool, c: Category) -> &'static str {
    match c {
        Category::PowerUp => tr(es, "power-up", "power-up"),
        Category::Enemy => tr(es, "enemy", "enemigo"),
        Category::Trap => tr(es, "trap", "trampa"),
        Category::Platform => tr(es, "platform", "plataforma"),
        Category::Goal => tr(es, "goal", "meta"),
        Category::Marker => tr(es, "invisible marker", "marcador invisible"),
        Category::Scripted => tr(es, "boss or scripted", "jefe o guion"),
    }
}

impl Editor {
    pub(super) fn open_catalogue(&mut self) {
        self.catalogue = Some(CatalogueView { objects: Vec::new(), filter: None, usable_only: false, chosen: None, scroll: 0.0 });
        self.status = tr(self.es, "Object catalogue: the game's objects. Esc: close.", "Catalogo de objetos: los objetos del juego. Esc: cerrar.").into();
    }

    /// The catalogue open (only power-ups with `power_ups`) once every object of the game's
    /// levels has its picture (for the check pictures).
    pub fn show_catalogue(&mut self, power_ups: bool) {
        let want: Vec<(usize, u8)> = (0..cart_model::levels::LEVELS).flat_map(|l| cart_model::objects::read(&self.doc.original, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        self.look_loader.prefer(&want);
        while want.iter().any(|key| !self.looks.contains_key(key)) {
            let Some((l, k, look)) = self.look_loader.wait() else { break };
            self.add_look(l, k, look);
        }
        self.open_catalogue();
        if let Some(v) = &mut self.catalogue {
            v.filter = power_ups.then_some(Category::PowerUp);
        }
    }

    /// The level shown's game level and whether it is in the second object catalogue.
    fn catalogue_of_shown(&self) -> (usize, bool) {
        let base = self.doc.base_of(self.cur);
        (base, base >= SECOND)
    }

    /// The catalogue's frame, in place of the level editor's.
    pub(super) fn catalogue_frame(&mut self) {
        let objects = self.catalogue_objects().to_vec();
        if let Some(v) = &mut self.catalogue {
            v.objects = objects;
        }
        self.pump_looks();
        let (_, second) = self.catalogue_of_shown();
        let offered = self.offered_here();
        let es = self.es;
        let Some(v) = self.catalogue.take() else { return };
        let mut v = v;
        clear_background(col(3));
        // which objects show, and which the level draws right (the type it has here)
        let here = |o: &CatalogueObject| o.uses.iter().find(|u| (u.0 >= SECOND) == second && offered.contains(&u.1)).map(|u| u.1);
        let shown: Vec<usize> = (0..v.objects.len())
            .filter(|&i| {
                let o = &v.objects[i];
                let cat = object_names::name(o.first().0, o.first().1).map(|n| n.1);
                v.filter.is_none_or(|f| cat == Some(f)) && (!v.usable_only || here(o).is_some())
            })
            .collect();
        // the side panel
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let w = self.panel - 2.0 * PAD - 8.0;
        draw_text(tr(es, "OBJECT CATALOGUE", "CATALOGO DE OBJETOS"), PAD, 25.0, 20.0, col(3));
        let mut y = 40.0;
        let half = (w - 4.0) / 2.0;
        if button(Rect::new(PAD, y, half, ROW), tr(es, "All", "Todos"), v.filter.is_none(), 15.0) {
            v.filter = None;
        }
        if button(Rect::new(PAD + half + 4.0, y, half, ROW), tr(es, "Usable here", "Usables aqui"), v.usable_only, 15.0) {
            v.usable_only = !v.usable_only;
        }
        y += ROW + 4.0;
        for (i, c) in CATEGORIES.iter().enumerate() {
            let r = Rect::new(PAD + (i % 2) as f32 * (half + 4.0), y + (i / 2) as f32 * (ROW + 4.0), half, ROW);
            if button(r, category_name(es, *c), v.filter == Some(*c), 14.0) {
                v.filter = Some(*c);
            }
        }
        y += CATEGORIES.len().div_ceil(2) as f32 * (ROW + 4.0) + 8.0;
        draw_text(&format!("{} {}", shown.len(), tr(es, "objects", "objetos")), PAD, y + 12.0, 15.0, col(2));
        y += 24.0;
        let mut pick = None;
        let mut add = None;
        let mut remove = None;
        let mut blank_from = None;
        let members = self.set_with_tiles();
        if let Some(o) = v.chosen.and_then(|i| v.objects.get(i)) {
            let (l, k) = o.first();
            let (name, cat) = object_names::name(l, k).unwrap_or(("?", Category::Marker));
            draw_text(&fit(name, w, 18.0), PAD, y + 14.0, 18.0, col(3));
            y += 22.0;
            for line in wrap(&self.object_details(o), 34) {
                draw_text(&line, PAD, y + 12.0, 15.0, col(3));
                y += 17.0;
            }
            y += 6.0;
            // brought into this level's enemies from the catalogue: it can be taken out again
            let imported = o.uses.iter().find_map(|&(l, k)| self.imported_enemy(k).filter(|e| e.level as usize == l));
            if let Some(e) = imported {
                let n = self.doc.levels[self.cur].content.objs.iter().filter(|ob| ob.kind == e.kind).count();
                if button(Rect::new(PAD, y, w, ROW), &format!("{} ({n} {})", tr(es, "Remove from this level", "Quitar de este nivel"), tr(es, "placed", "colocados")), false, 15.0) {
                    remove = Some(e);
                }
                y += ROW + 4.0;
            }
            match here(o) {
                Some(k) => {
                    if button(Rect::new(PAD, y, w, ROW), tr(es, "Use in this level", "Usar en este nivel"), false, 15.0) {
                        pick = Some(k);
                    }
                }
                None if cat == Category::Enemy => {
                    if let Some(e) = self.joinable(o, second, false) {
                        let label = tr(es, "Add to this level's enemies", "Anadir a los enemigos del nivel");
                        let r = Rect::new(PAD, y, w, ROW);
                        let fit = self.fit_with(e, members.as_deref());
                        if fit == Fit::Yes {
                            if button(r, label, false, 15.0) {
                                add = Some(e);
                            }
                        } else {
                            draw_rectangle(r.x, r.y, r.w, r.h, col(1));
                            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col(2));
                            let d = measure_text(label, None, 15, 1.0);
                            draw_text(label, r.x + (r.w - d.width) / 2.0, r.y + 17.0, 15.0, col(2));
                        }
                        y += ROW + 4.0;
                        let (why, kind) = match fit {
                            Fit::Yes => (tr(es, "Fits: its tiles are free in this level.", "Cabe: sus tiles estan libres en este nivel.").to_string(), Severity::Done),
                            Fit::Clash(other) => (format!("{} {} {}", tr(es, "Does not fit: it needs the tiles of", "No cabe: necesita los tiles de"), Self::enemy_name(other), tr(es, "(remove it first).", "(quitalo antes).")), Severity::Warning),
                            Fit::Full => (format!("{} {} {}.", tr(es, "Does not fit: the set has", "No cabe: el juego ya tiene"), enemy_sets::MOST, tr(es, "enemies", "enemigos")), Severity::Warning),
                            Fit::Pending => (format!("{} ({} {}).", tr(es, "Known once the enemies' tiles are captured", "Se sabra cuando se capturen los tiles de los enemigos"), self.look_loader.tiles_left(), tr(es, "left", "por capturar")), Severity::Info),
                        };
                        y += message_lines(&why, PAD, y + 12.0, w, 14.0, kind);
                        y -= ROW + 4.0;
                    } else {
                        // of the other object catalogue: say why, and where it can be used
                        let theirs = if second { "0-9" } else { "10-31" };
                        let mine = if second { "10-31" } else { "0-9" };
                        let why = format!(
                            "{} {theirs} {} {mine} {} {theirs}.",
                            tr(es, "Not usable here: it belongs to the object catalogue of levels", "No se usa aqui: es del catalogo de objetos de los niveles"),
                            tr(es, "and this level plays as one of levels", "y este nivel se juega como uno de los niveles"),
                            tr(es, "(a level uses one catalogue, and a type number is another object in the other). It can be used in a level, or a blank level, made from levels", "(un nivel usa un solo catalogo, y el mismo numero es otro objeto en el otro). Se puede usar en un nivel, o un nivel en blanco, hecho de los niveles")
                        );
                        y += message_lines(&why, PAD, y + 12.0, w, 14.0, Severity::Info) + 4.0;
                        if button(Rect::new(PAD, y, w, ROW), &format!("{} {theirs}", tr(es, "New blank level from levels", "Nivel en blanco de los niveles")), false, 14.0) {
                            blank_from = o.levels().first().copied();
                        }
                    }
                }
                None => {
                    y += message_lines(tr(es, "This level does not draw it right (its graphics are not loaded here).", "Este nivel no lo dibuja bien (sus graficos no se cargan aqui)."), PAD, y + 12.0, w, 14.0, Severity::Info) - ROW + 4.0;
                }
            }
            y += ROW + 8.0;
        }
        // what the last action did
        y += message_lines(&self.status.clone(), PAD, y + 12.0, w, 15.0, self.severity());
        y += 6.0;
        if button(Rect::new(PAD, y, w, ROW), &format!("{} ({})", tr(es, "Close", "Cerrar"), keys(Action::ObjectCatalogue)), false, 15.0) || is_key_pressed(KeyCode::Escape) {
            self.catalogue = None;
            return;
        }
        // the view: the objects in a grid
        let x0 = self.panel + 12.0;
        let per_row = (((screen_width() - x0 - 8.0) / CELL.0).floor() as usize).max(1);
        let rows = shown.len().div_ceil(per_row);
        let view_h = screen_height() - 30.0;
        let m: Vec2 = mouse_position().into();
        if m.x > self.panel {
            let wheel = mouse_wheel().1;
            v.scroll = (v.scroll - wheel.signum() * CELL.1 * (wheel != 0.0) as u8 as f32).clamp(0.0, (rows as f32 * CELL.1 - view_h + 12.0).max(0.0));
        }
        for (n, &i) in shown.iter().enumerate() {
            let o = &v.objects[i];
            let r = Rect::new(x0 + (n % per_row) as f32 * CELL.0, 8.0 + (n / per_row) as f32 * CELL.1 - v.scroll, CELL.0 - 6.0, CELL.1 - 6.0);
            if r.y + r.h < 0.0 || r.y > view_h {
                continue;
            }
            let usable = here(o).is_some();
            draw_rectangle(r.x, r.y, r.w, r.h, if v.chosen == Some(i) { col(1) } else { col(0) });
            if !usable {
                draw_rectangle_lines(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, Color::new(0.5, 0.5, 0.5, 1.0));
            }
            // an enemy of another level: whether it can join this level's set
            if let Some(e) = self.joinable(o, second, usable) {
                let (badge, kind) = match self.fit_with(e, members.as_deref()) {
                    Fit::Yes => (tr(es, "fits", "cabe"), Severity::Done),
                    Fit::Clash(_) | Fit::Full => (tr(es, "no room", "no cabe"), Severity::Warning),
                    Fit::Pending => ("...", Severity::Info),
                };
                let bw = measure_text(badge, None, 13, 1.0).width + 18.0;
                message(badge, r.x + r.w - bw, r.y + r.h - 6.0, 13.0, kind);
            }
            let (l, k) = o.first();
            if let Some((tex, _, _)) = self.looks.get(&(l, k)).and_then(|t| t.as_ref()) {
                let f = (40.0 / tex.width()).min(40.0 / tex.height()).min(2.0);
                draw_texture_ex(tex, r.x + 4.0, r.y + 4.0, WHITE, DrawTextureParams { dest_size: Some(vec2(tex.width() * f, tex.height() * f)), ..Default::default() });
            }
            let (name, cat) = object_names::name(l, k).map_or((format!("#{k:02X}"), "".to_string()), |(n, c)| (n.to_string(), category_name(es, c).to_string()));
            // the name on up to two lines, then the category
            let per_line = ((r.w - 52.0) / 7.5).max(6.0) as usize;
            let lines = wrap(&name, per_line);
            for (i, line) in lines.iter().take(2).enumerate() {
                draw_text(&fit(line, r.w - 50.0, 15.0), r.x + 48.0, r.y + 16.0 + i as f32 * 15.0, 15.0, col(3));
            }
            draw_text(&fit(&cat, r.w - 50.0, 13.0), r.x + 48.0, r.y + 18.0 + lines.len().min(2) as f32 * 15.0, 13.0, col(2));
            let levels: Vec<String> = o.levels().iter().map(|l| format!("{l:02}")).collect();
            draw_text(&fit(&levels.join(" "), r.w - 8.0, 13.0), r.x + 4.0, r.y + r.h - 6.0, 13.0, col(2));
            if r.contains(m) && is_mouse_button_pressed(MouseButton::Left) {
                v.chosen = Some(i);
            }
        }
        self.catalogue = Some(v);
        if self.own_pictures_pending() {
            let t = tr(es, "Still capturing the objects' pictures: until it ends some objects are missing or show twice, and Usable here may leave some out.", "Aun capturando los dibujos de los objetos: hasta que acabe faltan algunos o salen repetidos, y Usables aqui puede dejar fuera alguno.");
            let lines = wrap(t, ((screen_width() - x0 - 24.0) / 8.0) as usize);
            let h = lines.len() as f32 * 17.0 + 10.0;
            draw_rectangle(x0, view_h - h - 34.0, screen_width() - x0 - 10.0, h, col(0));
            draw_rectangle_lines(x0, view_h - h - 34.0, screen_width() - x0 - 10.0, h, 2.0, col(3));
            for (i, line) in lines.iter().enumerate() {
                draw_text(line, x0 + 8.0, view_h - h - 34.0 + 18.0 + i as f32 * 17.0, 15.0, col(3));
            }
        }
        draw_rectangle(self.panel, screen_height() - 22.0, screen_width() - self.panel, 22.0, col(1));
        status_message(&self.status, self.panel, screen_width(), screen_height() - 22.0, 22.0, 16.0, self.severity());
        self.capture_badge(screen_height() - 22.0);
        if let Some(k) = pick {
            self.catalogue = None;
            self.kind = None;
            self.choose_kind(k);
        }
        if let Some(e) = add {
            self.add_enemy(e);
        }
        if let Some(e) = remove {
            self.remove_enemy(e);
        }
        if let Some(l) = blank_from {
            self.catalogue = None;
            self.show_blank_chooser_at(l);
        }
    }

    /// An object's type numbers, levels and enemy tiles, for the catalogue's side panel.
    fn object_details(&self, o: &CatalogueObject) -> String {
        let es = self.es;
        let mut types: Vec<String> = o.uses.iter().map(|u| format!("${:02X} ({})", u.1, if u.0 >= SECOND { "10-31" } else { "0-9" })).collect();
        types.dedup();
        let levels: Vec<String> = o.levels().iter().map(|l| format!("{l:02}")).collect();
        let tiles = o.uses.iter().find_map(|u| self.object_tiles.get(u)).map(|t| enemy_sets::area_tiles(t).len());
        let mut s = format!("{}: {}. {}: {}.", tr(es, "Type", "Tipo"), types.join(", "), tr(es, "Levels", "Niveles"), levels.join(" "));
        if let Some(n) = tiles {
            s += &format!(" {} {n} {}.", tr(es, "Uses", "Usa"), tr(es, "of the 56 enemy tiles", "de los 56 tiles de enemigos"));
        }
        s
    }

    /// The enemies of game level `base` (types whose tiles there are in the enemy area), with
    /// their tiles; None while some are not captured yet.
    fn own_enemies(&self, base: usize) -> Option<Vec<(Enemy, Vec<u8>)>> {
        let mut kinds: Vec<u8> = cart_model::objects::read(&self.doc.original, base).0.iter().map(|o| o.kind).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let mut v = Vec::new();
        for k in kinds {
            let t = self.object_tiles.get(&(base, k))?;
            if !enemy_sets::area_tiles(t).is_empty() {
                v.push((Enemy { level: base as u8, kind: k }, t.clone()));
            }
        }
        Some(v)
    }

    /// The shown level's enemy set with its members' tiles (its own enemies when it has none).
    pub(super) fn set_with_tiles(&self) -> Option<Vec<(Enemy, Vec<u8>)>> {
        let base = self.doc.base_of(self.cur);
        match self.doc.level_enemies(self.cur) {
            Some(set) => set.enemies.iter().map(|e| Some((*e, self.object_tiles.get(&(e.level as usize, e.kind))?.clone()))).collect(),
            None => self.own_enemies(base),
        }
    }

    fn enemy_name(e: Enemy) -> String {
        object_names::name(e.level as usize, e.kind).map_or(format!("#{:02X}", e.kind), |n| n.0.to_string())
    }

    /// Whether enemy `e` can join the shown level's set, given the set with its tiles.
    fn fit_with(&self, e: Enemy, members: Option<&[(Enemy, Vec<u8>)]>) -> Fit {
        let (Some(members), Some(tiles)) = (members, self.object_tiles.get(&(e.level as usize, e.kind))) else { return Fit::Pending };
        if members.len() >= enemy_sets::MOST {
            return Fit::Full;
        }
        members.iter().find(|m| enemy_sets::clash(tiles, &m.1)).map_or(Fit::Yes, |m| Fit::Clash(m.0))
    }

    /// The enemy of the shown level's catalogue an object is there, if it is an enemy that
    /// level does not draw right yet.
    fn joinable(&self, o: &CatalogueObject, second: bool, usable: bool) -> Option<Enemy> {
        let (l, k) = o.first();
        let enemy = object_names::name(l, k).is_some_and(|n| n.1 == Category::Enemy);
        if !enemy || usable {
            return None;
        }
        o.uses.iter().find(|u| (u.0 >= SECOND) == second).map(|&(l, k)| Enemy { level: l as u8, kind: k })
    }

    /// Put enemy `e` in the shown level's set, if its tiles are known and fit.
    pub(super) fn add_enemy(&mut self, e: Enemy) {
        let es = self.es;
        let (Some(members), Some(tiles)) = (self.set_with_tiles(), self.object_tiles.get(&(e.level as usize, e.kind)).cloned()) else {
            self.status = format!("{} ({} {}).", tr(es, "Not added: the enemies' tiles are still being captured; try again in a moment", "No anadido: los tiles de los enemigos aun se estan capturando; prueba en un momento"), self.look_loader.tiles_left(), tr(es, "left", "por capturar"));
            self.mark(Severity::Warning);
            return;
        };
        if members.iter().any(|m| m.0 == e) {
            return;
        }
        if members.len() >= enemy_sets::MOST {
            self.status = format!("{} {} {}.", tr(es, "Not added: a level's set has at most", "No anadido: el juego de un nivel tiene como mucho"), enemy_sets::MOST, tr(es, "enemies", "enemigos"));
            self.mark(Severity::Warning);
            return;
        }
        if let Some((other, _)) = members.iter().find(|m| enemy_sets::clash(&tiles, &m.1)) {
            self.status = format!("{} {} {} {}.", Self::enemy_name(e), tr(es, "does not fit: it needs the tiles of", "no cabe: necesita los tiles de"), Self::enemy_name(*other), tr(es, "(remove it first in the Level tab)", "(quitalo antes en la pestana Nivel)"));
            self.mark(Severity::Warning);
            return;
        }
        let mut all = members;
        all.push((e, tiles));
        self.set_level_enemies_from(all);
        self.status = format!("{} {}.", Self::enemy_name(e), tr(es, "added to this level's enemies", "anadido a los enemigos del nivel"));
        self.mark(Severity::Done);
    }

    /// Take enemy `e` out of the shown level's set, and its objects out of the level (they
    /// would look broken), in one undo step.
    pub(super) fn remove_enemy(&mut self, e: Enemy) {
        let es = self.es;
        let Some(mut members) = self.set_with_tiles() else {
            self.status = tr(es, "Not removed: the enemies' tiles are still being captured.", "No quitado: los tiles de los enemigos aun se estan capturando.").into();
            self.mark(Severity::Warning);
            return;
        };
        members.retain(|m| m.0 != e);
        let base = self.doc.base_of(self.cur);
        let block = enemy_sets::build(&self.doc.original, base, &members);
        let set = EnemySet { enemies: members.iter().map(|m| m.0).collect(), block };
        let l = self.cur;
        let gone = self.doc.drop_set_enemy(l, set, e.kind);
        self.offered.clear();
        self.sel_obj = None;
        if self.kind == Some(e.kind) {
            self.kind = None;
        }
        self.status = format!("{} {}; {gone} {} {}", Self::enemy_name(e), tr(es, "removed from this level's enemies", "quitado de los enemigos del nivel"), tr(es, "of its objects removed from the level", "de sus objetos quitados del nivel"), tr(es, "(Ctrl+Z brings them back).", "(Ctrl+Z los recupera)."));
        self.mark(Severity::Done);
    }

    /// The enemy of the shown level's set that came from another level and is type `k`.
    pub(super) fn imported_enemy(&self, k: u8) -> Option<Enemy> {
        let base = self.doc.base_of(self.cur) as u8;
        self.doc.level_enemies(self.cur)?.enemies.iter().find(|e| e.kind == k && e.level != base).copied()
    }

    fn set_level_enemies_from(&mut self, members: Vec<(Enemy, Vec<u8>)>) {
        let base = self.doc.base_of(self.cur);
        let block = enemy_sets::build(&self.doc.original, base, &members);
        let set = EnemySet { enemies: members.iter().map(|m| m.0).collect(), block };
        let l = self.cur;
        self.doc.set_level_enemies(l, Some(set));
        self.offered.clear();
    }

    /// The shown level goes back to the game's enemies.
    pub(super) fn game_enemies(&mut self) {
        let l = self.cur;
        self.doc.set_level_enemies(l, None);
        self.offered.clear();
    }

    /// The names of the shown level's set (None: the game's).
    pub(super) fn enemy_rows(&self) -> Option<Vec<(Enemy, String)>> {
        let set = self.doc.level_enemies(self.cur)?;
        Some(set.enemies.iter().map(|&e| (e, format!("{} ({:02})", Self::enemy_name(e), e.level))).collect())
    }
}
