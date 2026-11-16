//! The level editor's side panel. A fixed head: the level picker (its grid folds away), a
//! toolbar (undo, redo, save, play, play here), the Blocks / Objects switch with its tools,
//! and two tabs. Below them the chosen tab fills the rest: the palette, or the level's
//! inspector (level, goals, settings, warnings), which scrolls with the wheel when it does not
//! fit. Buttons carry short names; their keys show in a tooltip.
use super::names_view::Named;
use super::shortcuts::{keys, Action};
use super::start_view::Placing;
use super::testing_view::warn_col;
use super::widgets::*;
use super::{Editor, Mode};
use crate::domain::document::CheckpointRefused;
use crate::domain::prefs::MUSIC_VOLUMES;
use crate::domain::tools::ToolKind;
use cart_model::checkpoint;
use cart_model::level_goals::Goal;
use cart_model::level_settings::{MUSICS, PALETTES, TIME_MAX, TIME_MIN};
use cart_model::levels::LEVELS;
use macroquad::prelude::*;

/// What the side panel shows below its head.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SideTab {
    Palette,
    Level,
}

const PAD: f32 = 10.0;
const ROW: f32 = 26.0;
const GAP: f32 = 4.0;
/// The inspector's label column.
const LABEL_W: f32 = 96.0;
const LEVEL_BUTTON: (f32, f32) = (33.0, 23.0);
/// Seconds a warning's place stays marked after it is clicked.
const MARK_SECONDS: f64 = 1.5;
pub(super) const SCROLL_STEP: f32 = 40.0;

/// Rows laid out down the inspector, drawn only while inside it.
struct Rows {
    y: f32,
    top: f32,
    bottom: f32,
}

impl Rows {
    fn next(&mut self, h: f32) -> Option<f32> {
        let y = self.y;
        self.y += h + GAP;
        (y >= self.top - 0.5 && y + h <= self.bottom).then_some(y)
    }
}

/// `t` cut to fit `w` pixels at `size`.
pub(super) fn fit(t: &str, w: f32, size: f32) -> String {
    let width = |s: &str| measure_text(s, None, size as u16, 1.0).width;
    if width(t) <= w {
        return t.to_string();
    }
    let mut s = t.to_string();
    while !s.is_empty() && width(&format!("{s}..")) > w {
        s.pop();
    }
    format!("{s}..")
}

impl Editor {
    fn inner_w(&self) -> f32 {
        self.panel - 2.0 * PAD - 3.0
    }

    /// A button whose name and keys show in a tooltip while the mouse is on it.
    fn tool_button(&mut self, r: Rect, name: &str, a: Action, on: bool) -> bool {
        if r.contains(mouse_position().into()) {
            let k = keys(a);
            self.tip = Some(if k.is_empty() { name.to_string() } else { format!("{name}  ({k})") });
        }
        button(r, name, on, 15.0)
    }

    /// The tooltip of the button under the mouse, over everything.
    pub(super) fn draw_tip(&mut self) {
        let Some(t) = self.tip.take() else { return };
        let m: Vec2 = mouse_position().into();
        let d = measure_text(&t, None, 15, 1.0);
        let (w, h) = (d.width + 12.0, 22.0);
        let x = (m.x + 14.0).min(screen_width() - w - 4.0);
        let y = (m.y + 18.0).min(screen_height() - h - 4.0);
        draw_rectangle(x, y, w, h, col(3));
        draw_rectangle_lines(x, y, w, h, 1.0, col(1));
        draw_text(&t, x + 6.0, y + 15.0, 15.0, col(0));
    }

    /// Level buttons, toolbar, mode, tools, tabs and the chosen tab. Returns the action
    /// chosen (done after drawing) and the level picked.
    pub(super) fn level_panel(&mut self) -> (Option<Action>, Option<usize>) {
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let es = self.es;
        let w = self.inner_w();
        let mut act = None;
        let mut pick = None;
        draw_text(tr(es, "LEVEL EDITOR", "EDITOR DE NIVELES"), PAD, 25.0, 20.0, col(3));
        if self.tool_button(Rect::new(PAD + w - 104.0, 8.0, 104.0, 24.0), tr(es, "World map", "Mapa mundial"), Action::WorldEditor, false) {
            act = Some(Action::WorldEditor);
        }
        let mut y = 40.0;
        // the level picker: < Level 07 · name v >
        if self.tool_button(Rect::new(PAD, y, 30.0, ROW), "<", Action::PrevLevel, false) {
            act = Some(Action::PrevLevel);
        }
        if self.tool_button(Rect::new(PAD + w - 30.0, y, 30.0, ROW), ">", Action::NextLevel, false) {
            act = Some(Action::NextLevel);
        }
        let lv = &self.doc.levels[self.cur];
        let mut label = format!("{} {:02}", tr(es, "Level", "Nivel"), self.cur);
        if !lv.content.name.is_empty() {
            label += &format!("  {}", lv.content.name);
        }
        if lv.dirty() {
            label += " *";
        }
        if self.doc.mods.level(self.cur) {
            label += "  mod";
        }
        let mid = Rect::new(PAD + 34.0, y, w - 68.0, ROW);
        let shown = format!("{}  {}", fit(&label, mid.w - 30.0, 15.0), if self.levels_open { "^" } else { "v" });
        if button(mid, &shown, self.levels_open, 15.0) {
            self.levels_open = !self.levels_open;
        }
        y += ROW + GAP;
        if self.levels_open {
            let per_row = ((w + 2.0) / (LEVEL_BUTTON.0 + 2.0)).floor().max(1.0) as usize;
            let count = self.doc.levels.len();
            for l in 0..count {
                let (cx, cy) = (PAD + (l % per_row) as f32 * (LEVEL_BUTTON.0 + 2.0), y + (l / per_row) as f32 * (LEVEL_BUTTON.1 + 2.0));
                let label = format!("{:02}{}", l, if self.doc.levels[l].dirty() { "*" } else { "" });
                if button(Rect::new(cx, cy, LEVEL_BUTTON.0, LEVEL_BUTTON.1), &label, l == self.cur, 15.0) {
                    pick = Some(l);
                }
                if self.doc.mods.level(l) {
                    draw_triangle(vec2(cx + LEVEL_BUTTON.0, cy), vec2(cx + LEVEL_BUTTON.0 - 8.0, cy), vec2(cx + LEVEL_BUTTON.0, cy + 8.0), col(2));
                }
                let n = self.doc.warnings(l).len();
                if n > 0 {
                    draw_circle(cx + LEVEL_BUTTON.0 - 5.0, cy + LEVEL_BUTTON.1 - 5.0, 5.0, warn_col());
                    draw_text(&n.to_string(), cx + LEVEL_BUTTON.0 - 8.0, cy + LEVEL_BUTTON.1 - 1.0, 12.0, BLACK);
                }
            }
            y += count.div_ceil(per_row) as f32 * (LEVEL_BUTTON.1 + 2.0) + 2.0;
            let third = (w - 2.0 * GAP) / 3.0;
            if self.tool_button(Rect::new(PAD, y, third, ROW), tr(es, "Copy level", "Copiar nivel"), Action::NewLevel, false) {
                act = Some(Action::NewLevel);
            }
            if self.tool_button(Rect::new(PAD + third + GAP, y, third, ROW), tr(es, "Blank level", "Nivel en blanco"), Action::NewBlankLevel, false) {
                act = Some(Action::NewBlankLevel);
            }
            if self.cur >= LEVELS && self.tool_button(Rect::new(PAD + 2.0 * (third + GAP), y, third, ROW), tr(es, "Delete level", "Borrar nivel"), Action::DeleteLevel, false) {
                act = Some(Action::DeleteLevel);
            }
            y += ROW + GAP;
        }
        // toolbar: history | file and game
        y += 4.0;
        let bw = (w - 4.0 * GAP - 10.0) / 5.0;
        let tools = [
            (tr(es, "Undo", "Deshacer"), Action::Undo, false, 0.0),
            (tr(es, "Redo", "Rehacer"), Action::Redo, false, 0.0),
            (tr(es, "Save", "Guardar"), Action::Save, false, 10.0),
            (tr(es, "Play", "Jugar"), Action::Play, false, 10.0),
            (tr(es, "Here", "Aqui"), Action::PlayFromHere, self.placing == Some(Placing::PlayHere), 10.0),
        ];
        for (i, (name, a, on, shift)) in tools.into_iter().enumerate() {
            let x = PAD + i as f32 * (bw + GAP) + shift;
            if self.tool_button(Rect::new(x, y, bw, ROW), name, a, on) {
                act = Some(a);
            }
        }
        y += ROW + GAP + 6.0;
        // Blocks / Objects, then the mode's tools
        let half = (w - GAP) / 2.0;
        let blocks = self.mode == Mode::Blocks;
        if self.tool_button(Rect::new(PAD, y, half, ROW), tr(es, "Blocks", "Bloques"), Action::BlocksObjects, blocks) {
            self.set_mode(Mode::Blocks);
        }
        if self.tool_button(Rect::new(PAD + half + GAP, y, half, ROW), tr(es, "Objects", "Objetos"), Action::BlocksObjects, !blocks) {
            self.set_mode(Mode::Objects);
        }
        y += ROW + GAP;
        let qw = (w - 3.0 * GAP) / 4.0;
        if blocks {
            let tool = self.tool.kind();
            for (i, (name, a, k)) in [(tr(es, "Pencil", "Lapiz"), Action::Pencil, ToolKind::Pencil), (tr(es, "Box", "Caja"), Action::BoxFill, ToolKind::Box), (tr(es, "Select", "Seleccion"), Action::Select, ToolKind::Select)].into_iter().enumerate() {
                if self.tool_button(Rect::new(PAD + i as f32 * (qw + GAP), y, qw, ROW), name, a, tool == k) {
                    act = Some(a);
                }
            }
        } else {
            let n = self.doc.levels[self.cur].content.objs.len();
            draw_text(&format!("{n} {}", tr(es, "objects", "objetos")), PAD + 2.0, y + 18.0, 16.0, col(3));
        }
        if self.tool_button(Rect::new(PAD + 3.0 * (qw + GAP), y, qw, ROW), tr(es, "Grid", "Rejilla"), Action::Grid, self.grid) {
            act = Some(Action::Grid);
        }
        y += ROW + GAP + 6.0;
        // tabs
        let warnings = self.doc.warnings(self.cur).len();
        let level_tab = if warnings > 0 { format!("{}  ! {warnings}", tr(es, "Level", "Nivel")) } else { tr(es, "Level", "Nivel").to_string() };
        for (i, (name, tab)) in [(tr(es, "Palette", "Paleta").to_string(), SideTab::Palette), (level_tab, SideTab::Level)].into_iter().enumerate() {
            let r = Rect::new(PAD + i as f32 * (half + GAP), y, half, ROW);
            if button(r, &name, self.side_tab == tab, 15.0) {
                self.side_tab = tab;
            }
            if tab == SideTab::Level && warnings > 0 && self.side_tab != tab {
                draw_circle(r.x + r.w - 8.0, r.y + 8.0, 4.0, warn_col());
            }
        }
        y += ROW;
        draw_rectangle(PAD, y, w, 2.0, col(3));
        y += 8.0;
        match self.side_tab {
            SideTab::Palette if self.mode == Mode::Objects => self.object_palette(y),
            SideTab::Palette => self.block_palette(y),
            SideTab::Level => {
                if let Some(a) = self.inspector(y) {
                    act = Some(a);
                }
            }
        }
        (act, pick)
    }

    fn heading(&self, rows: &mut Rows, text: &str) {
        let Some(y) = rows.next(18.0) else { return };
        draw_text(text, PAD, y + 13.0, 14.0, col(2));
        let d = measure_text(text, None, 14, 1.0);
        draw_rectangle(PAD + d.width + 8.0, y + 8.0, self.inner_w() - d.width - 8.0, 1.0, col(1));
    }

    fn label(y: f32, text: &str) {
        draw_text(text, PAD, y + 18.0, 15.0, col(3));
    }

    /// A note over as many rows as it needs.
    fn explain(rows: &mut Rows, w: f32, text: &str) {
        let per_line = ((w / 7.5) as usize).max(10);
        for line in wrap(text, per_line) {
            if let Some(y) = rows.next(16.0) {
                draw_text(&line, PAD, y + 12.0, 14.0, col(2));
            }
        }
    }

    fn note(x: f32, y: f32, w: f32, text: &str) {
        draw_text(&fit(text, w, 14.0), x, y + 18.0, 14.0, col(2));
    }

    /// The shown level's inspector: level, goals, settings and warnings.
    fn inspector(&mut self, top: f32) -> Option<Action> {
        let es = self.es;
        let w = self.inner_w();
        let bottom = screen_height() - 6.0;
        let m: Vec2 = mouse_position().into();
        if m.x < self.panel && m.y > top {
            let (_, wheel) = mouse_wheel();
            let most = (self.side_content - (bottom - top)).max(0.0);
            self.side_scroll = (self.side_scroll - wheel.signum() * SCROLL_STEP * (wheel != 0.0) as u8 as f32).clamp(0.0, most);
        }
        let mut rows = Rows { y: top - self.side_scroll, top, bottom };
        let mut act = None;
        let (cx, cw) = (PAD + LABEL_W, w - LABEL_W);
        let l = self.cur;

        self.heading(&mut rows, tr(es, "LEVEL", "NIVEL"));
        if l >= LEVELS {
            if let Some(y) = rows.next(ROW) {
                self.name_field(Rect::new(PAD, y, w, ROW), Named::Level(l));
            }
            if let Some(y) = rows.next(ROW) {
                Self::label(y, tr(es, "Plays as", "Se juega como"));
                Self::note(cx, y, cw, &format!("{} {:02}", tr(es, "level", "nivel"), self.doc.base_of(l)));
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Original", "Original"));
            if self.tool_button(Rect::new(cx, y, 90.0, ROW), tr(es, "Restore", "Restaurar"), Action::Restore, false) {
                act = Some(Action::Restore);
            }
            if self.doc.mods.level(l) {
                Self::note(cx + 96.0, y, cw - 96.0, tr(es, "mod (saved)", "mod (guardado)"));
            }
        }

        self.heading(&mut rows, tr(es, "GOALS", "METAS"));
        let place = 70.0;
        let start = self.doc.level_start(l);
        let goals = [
            (tr(es, "Start", "Inicio"), Action::PlaceStart, Placing::Start, format!("x {:04X} y {:04X}", start.mario_x, start.mario_y), true),
            (tr(es, "Finish", "Meta"), Action::PlaceFinish, Placing::Finish, self.goal_text(Goal::Finish), !self.doc.ends_with_boss(l)),
            (tr(es, "Secret exit", "Salida secreta"), Action::PlaceSecret, Placing::Secret, self.goal_text(Goal::Secret), true),
        ];
        for (name, a, p, text, can) in goals {
            if let Some(y) = rows.next(ROW) {
                Self::label(y, name);
                if can && self.tool_button(Rect::new(cx, y, place, ROW), tr(es, "Place", "Colocar"), a, self.placing == Some(p)) {
                    act = Some(a);
                }
                Self::note(cx + place + 6.0, y, cw - place - 6.0, &text);
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Leads to", "Lleva a"));
            if self.tool_button(Rect::new(cx, y, place, ROW), tr(es, "Choose", "Elegir"), Action::SecretLeads, false) {
                act = Some(Action::SecretLeads);
            }
            Self::note(cx + place + 6.0, y, cw - place - 6.0, &self.secret_leads());
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Checkpoints", "Controles"));
            let refused = self.doc.checkpoint_refused(l);
            if refused != Some(CheckpointRefused::NoRoom) && self.tool_button(Rect::new(cx, y, place, ROW), tr(es, "Add", "Poner"), Action::PlaceCheckpoint, self.placing == Some(Placing::Checkpoint)) {
                act = Some(Action::PlaceCheckpoint);
            }
            let n = checkpoint::bells(&self.doc.levels[l].content.objs).len();
            let t = match refused {
                Some(CheckpointRefused::NoRoom) => tr(es, "none in this level", "ninguno en este nivel").to_string(),
                _ => format!("{n} / {}", checkpoint::MAX),
            };
            Self::note(cx + place + 6.0, y, cw - place - 6.0, &t);
        }

        self.heading(&mut rows, tr(es, "SETTINGS", "AJUSTES"));
        let mut s = self.doc.level_settings(l);
        let small = 26.0;
        // a value between - and + (or < and >) buttons; returns -1, 0 or 1
        let stepper = |rows_y: f32, minus: &str, plus: &str, value: &str| -> i32 {
            let mut d = 0;
            if button(Rect::new(cx, rows_y, small, ROW), minus, false, 16.0) {
                d = -1;
            }
            let vw = cw - 2.0 * small - 2.0 * GAP;
            let t = fit(value, vw - 4.0, 15.0);
            let dim = measure_text(&t, None, 15, 1.0);
            draw_text(&t, cx + small + GAP + (vw - dim.width) / 2.0, rows_y + 18.0, 15.0, col(3));
            if button(Rect::new(cx + cw - small, rows_y, small, ROW), plus, false, 16.0) {
                d = 1;
            }
            d
        };
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Time", "Tiempo"));
            let d = stepper(y, "-", "+", &format!("{}", s.time as u32 * 100));
            s.time = (s.time as i32 + d).clamp(TIME_MIN as i32, TIME_MAX as i32) as u8;
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Background", "Fondo"));
            let names = [tr(es, "normal", "normal"), tr(es, "dark", "oscuro"), tr(es, "darker", "mas oscuro")];
            let i = PALETTES.iter().position(|&p| p == s.palette);
            let d = stepper(y, "<", ">", i.map_or("?", |i| names[i]));
            if d != 0 {
                s.palette = PALETTES[(i.unwrap_or(0) as i32 + d).rem_euclid(PALETTES.len() as i32) as usize];
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Music", "Musica"));
            let i = MUSICS.iter().position(|&(m, _)| m == s.music);
            let name = i.map_or(format!("${:02X}", s.music), |i| format!("{} {:02}", tr(es, "as level", "la del"), MUSICS[i].1));
            let d = stepper(y, "<", ">", &name);
            if d != 0 {
                s.music = MUSICS[(i.unwrap_or(0) as i32 + d).rem_euclid(MUSICS.len() as i32) as usize].0;
            }
        }
        let playing = self.music_playing();
        if let Some(y) = rows.next(ROW) {
            if button(Rect::new(PAD, y, LABEL_W - GAP, ROW), if playing { tr(es, "Stop", "Parar") } else { tr(es, "Listen", "Escuchar") }, playing, 15.0) {
                self.toggle_music(s.music);
            }
            // how loud Listen plays: the editor's own setting, not the game's volume
            let v = self.prefs.music_volume as i32;
            let d = stepper(y, "-", "+", &format!("{} {v}", tr(es, "editor vol", "vol. editor")));
            let nv = (v + d).clamp(*MUSIC_VOLUMES.start() as i32, *MUSIC_VOLUMES.end() as i32) as u32;
            if nv != self.prefs.music_volume {
                self.set_music_volume(nv);
            }
        }
        if s != self.doc.level_settings(l) {
            self.doc.set_level_settings(l, s);
            if playing {
                self.toggle_music(s.music);
                self.toggle_music(s.music);
            }
        }
        // the game's per-level tables: two choices each
        let game = self.doc.game_tables(l);
        let mut t = self.doc.level_tables(l);
        let choice = |y: f32, names: [&str; 2], on: bool| -> Option<bool> {
            let hw = (cw - GAP) / 2.0;
            let mut pick = None;
            for (i, name) in names.iter().enumerate() {
                if button(Rect::new(cx + i as f32 * (hw + GAP), y, hw, ROW), name, on == (i == 1), 15.0) {
                    pick = Some(i == 1);
                }
            }
            pick
        };
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Gravity", "Gravedad"));
            if let Some(on) = choice(y, [tr(es, "normal", "normal"), tr(es, "space", "espacio")], t.space_gravity()) {
                t.set_space_gravity(on, &game);
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Auto-scroll", "Auto-scroll"));
            if let Some(on) = choice(y, [tr(es, "off", "no"), tr(es, "on", "si")], t.scrolls()) {
                t.set_scrolls(on);
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(es, "Camera", "Camara"));
            if let Some(on) = choice(y, [tr(es, "normal", "normal"), tr(es, "wide", "amplia")], t.wide_camera()) {
                t.set_wide_camera(on);
            }
        }
        Self::explain(&mut rows, w, tr(es, "Wide: when Mario climbs or jumps high the camera goes up sooner, so more of what is above shows.", "Amplia: si Mario trepa o salta alto, la camara sube antes y se ve mas de lo que hay arriba."));
        if game.animated() {
            if let Some(y) = rows.next(ROW) {
                Self::label(y, tr(es, "Animation", "Animacion"));
                if let Some(on) = choice(y, [tr(es, "none", "ninguna"), tr(es, "its own", "la suya")], t.animated()) {
                    t.set_animated(on, &game);
                }
            }
        }
        if t.scrolls() {
            Self::explain(&mut rows, w, tr(es, "Auto-scroll: the screen moves right by itself; caught against a wall Mario loses a life.", "Auto-scroll: la pantalla avanza sola; atrapado contra una pared Mario pierde una vida."));
        }
        if t != self.doc.level_tables(l) {
            self.doc.set_level_tables(l, t);
        }

        self.heading(&mut rows, tr(es, "ENEMIES", "ENEMIGOS"));
        match self.enemy_rows() {
            None => {
                Self::explain(&mut rows, w, tr(es, "The game's enemies of the level it plays as. The object catalogue (F3) adds enemies of other levels that fit.", "Los enemigos del juego del nivel como el que se juega. El catalogo de objetos (F3) anade enemigos de otros niveles que quepan."));
            }
            Some(list) => {
                let mut out = None;
                // removing rebuilds the set's sprite block, which needs every member's tiles
                let ready = self.set_with_tiles().is_some();
                for (e, name) in list {
                    if let Some(y) = rows.next(ROW) {
                        draw_text(&fit(&name, cw + LABEL_W - 90.0, 15.0), PAD, y + 18.0, 15.0, col(3));
                        let r = Rect::new(PAD + w - 84.0, y, 84.0, ROW);
                        if !ready {
                            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col(2));
                            draw_text(tr(es, "Remove", "Quitar"), r.x + 18.0, r.y + 17.0, 14.0, col(2));
                        } else if button(r, tr(es, "Remove", "Quitar"), false, 14.0) {
                            out = Some(e);
                        }
                    }
                }
                if !ready {
                    Self::explain(&mut rows, w, tr(es, "Enemies can be removed once their tiles are captured (see the box at the bottom right).", "Los enemigos se pueden quitar cuando se capturen sus tiles (mira el recuadro de abajo a la derecha)."));
                }
                if let Some(e) = out {
                    self.remove_enemy(e);
                }
                if let Some(y) = rows.next(ROW) {
                    if button(Rect::new(PAD, y, w, ROW), tr(es, "The game's enemies again", "Volver a los enemigos del juego"), false, 14.0) {
                        self.game_enemies();
                    }
                }
            }
        }
        if let Some(y) = rows.next(ROW) {
            if button(Rect::new(PAD, y, w, ROW), &format!("{} ({})", tr(es, "Object catalogue", "Catalogo de objetos"), keys(Action::ObjectCatalogue)), false, 14.0) {
                act = Some(Action::ObjectCatalogue);
            }
        }

        let warnings = self.doc.warnings(l);
        self.heading(&mut rows, &format!("{} ({})", tr(es, "WARNINGS", "AVISOS"), warnings.len()));
        if warnings.is_empty() {
            if let Some(y) = rows.next(20.0) {
                draw_text(tr(es, "None: the level looks fine.", "Ninguno: el nivel parece correcto."), PAD, y + 14.0, 15.0, col(2));
            }
        }
        for wa in &warnings {
            let Some(y) = rows.next(20.0) else { continue };
            let r = Rect::new(PAD, y, w, 20.0);
            let place = wa.place();
            if r.contains(m) && place.is_some() {
                draw_rectangle(r.x, r.y, r.w, r.h, col(1));
                self.tip = Some(tr(es, "Click: show it on the level", "Clic: verlo en el nivel").into());
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (x, py) = place.unwrap_or_default();
                    let p = vec2(x as f32, py as f32);
                    self.look = Some((p, true));
                    self.mark = Some((p, get_time() + MARK_SECONDS));
                }
            }
            draw_circle(PAD + 6.0, y + 10.0, 4.0, warn_col());
            draw_text(&fit(&self.warning_text(wa), w - 18.0, 15.0), PAD + 16.0, y + 15.0, 15.0, col(3));
        }
        self.side_content = rows.y + self.side_scroll - top;
        if self.side_content > bottom - top {
            // where the view is in the whole inspector
            let frac = (bottom - top) / self.side_content;
            let at = self.side_scroll / self.side_content;
            draw_rectangle(self.panel - 8.0, top + at * (bottom - top), 3.0, frac * (bottom - top), col(2));
        }
        act
    }

    /// Where a goal is, or that the level has none.
    fn goal_text(&self, g: Goal) -> String {
        match self.doc.goal_cells(self.cur, g).first() {
            Some(&(c, r)) => format!("{c}, {r}"),
            None if g == Goal::Finish && self.doc.ends_with_boss(self.cur) => tr(self.es, "ends with its boss", "termina con su jefe").to_string(),
            None => tr(self.es, "none", "ninguna").to_string(),
        }
    }
}
