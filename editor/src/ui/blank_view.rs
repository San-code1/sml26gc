//! New blank level: one of the game's 32 levels is chosen while only its block palette and its
//! object palette are shown, as they look in that level; Create adds an empty level that plays
//! as it (`EditorDocument::add_blank_level`).
use super::palette::source;
use super::shortcuts::{keys, Action};
use super::level_panel::fit;
use super::widgets::*;
use super::Editor;
use cart_model::level_tables::Tables;
use cart_model::levels::LEVELS;
use macroquad::prelude::*;

const PAD: f32 = 10.0;
const ROW: f32 = 26.0;
const LEVEL_BUTTON: (f32, f32) = (33.0, 23.0);
/// Wario's final room: no map point opens it.
const WARIO_ROOM: usize = 24;
const CELL: f32 = 36.0;

impl Editor {
    pub(super) fn open_blank_chooser(&mut self) {
        self.blank_choice = Some(self.doc.base_of(self.cur).min(LEVELS - 1));
        self.status = tr(self.es, "New blank level: choose the level it plays as. Esc: cancel.", "Nivel en blanco: elige el nivel como el que se juega. Esc: cancelar.").into();
    }

    /// The chooser open on game level `base` (for the check pictures).
    pub fn show_blank_chooser(&mut self, base: usize) {
        self.open_blank_chooser();
        let base = base.min(LEVELS - 1);
        self.blank_choice = Some(base);
        let mut want: Vec<(usize, u8)> = (0..128u8).map(|k| (base, k)).collect();
        want.extend((0..128u8).flat_map(|k| cart_model::level_graphics::levels_using(&self.doc.original, base, k).into_iter().map(move |l| (l, k))));
        self.look_loader.prefer(&want);
        while want.iter().any(|key| !self.looks.contains_key(key)) {
            let Some((l, k, look)) = self.look_loader.wait() else { break };
            self.add_look(l, k, look);
        }
    }

    /// The chooser's frame, in place of the level editor's.
    pub(super) fn blank_frame(&mut self) {
        let Some(base) = self.blank_choice else { return };
        self.ensure_art(base);
        let want: Vec<(usize, u8)> = (0..128u8).map(|k| (base, k)).filter(|key| !self.looks.contains_key(key)).collect();
        self.look_loader.prefer(&want);
        for (l, k, look) in self.look_loader.take() {
            self.add_look(l, k, look);
        }
        clear_background(col(3));
        let mut choice = Some(base);
        let mut create = false;
        // the side panel: the 32 levels, what comes with the chosen one, Create and Cancel
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let es = self.es;
        let w = self.panel - 2.0 * PAD - 8.0;
        draw_text(tr(es, "NEW BLANK LEVEL", "NIVEL EN BLANCO"), PAD, 25.0, 20.0, col(3));
        draw_text(tr(es, "Plays as:", "Se juega como:"), PAD, 52.0, 15.0, col(2));
        let mut y = 60.0;
        let per_row = ((w + 2.0) / (LEVEL_BUTTON.0 + 2.0)).floor().max(1.0) as usize;
        for l in 0..LEVELS {
            let r = Rect::new(PAD + (l % per_row) as f32 * (LEVEL_BUTTON.0 + 2.0), y + (l / per_row) as f32 * (LEVEL_BUTTON.1 + 2.0), LEVEL_BUTTON.0, LEVEL_BUTTON.1);
            if button(r, &format!("{l:02}"), l == base, 15.0) {
                choice = Some(l);
            }
        }
        y += LEVELS.div_ceil(per_row) as f32 * (LEVEL_BUTTON.1 + 2.0) + 10.0;
        for line in self.blank_notes(base).iter().flat_map(|l| wrap(l, 34)) {
            draw_text(&line, PAD, y + 12.0, 15.0, col(3));
            y += 17.0;
        }
        y += 10.0;
        let half = (w - 4.0) / 2.0;
        if button(Rect::new(PAD, y, half, ROW), tr(es, "Create", "Crear"), false, 15.0) || is_key_pressed(KeyCode::Enter) {
            create = true;
        }
        if button(Rect::new(PAD + half + 4.0, y, half, ROW), tr(es, "Cancel (Esc)", "Cancelar (Esc)"), false, 15.0) || is_key_pressed(KeyCode::Escape) {
            choice = None;
        }
        // the view: the chosen level's blocks and objects
        let x0 = self.panel + 16.0;
        draw_text(&format!("{} {base:02}", tr(es, "BLOCKS OF LEVEL", "BLOQUES DEL NIVEL")), x0, 26.0, 18.0, col(0));
        if let Some(atlas) = self.atlases[base].clone() {
            for i in 0..128 {
                let r = Rect::new(x0 + (i % 8) as f32 * CELL, 36.0 + (i / 8) as f32 * CELL, CELL - 2.0, CELL - 2.0);
                draw_texture_ex(&atlas, r.x, r.y, WHITE, DrawTextureParams { source: Some(source(i, 16.0)), dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
            }
        }
        let x1 = x0 + 8.0 * CELL + 24.0;
        draw_text(&format!("{} {base:02}", tr(es, "OBJECTS OF LEVEL", "OBJETOS DEL NIVEL")), x1, 26.0, 18.0, col(0));
        let offered = self.offered_types(base);
        for (n, &k) in offered.iter().enumerate() {
            let r = Rect::new(x1 + (n % 8) as f32 * CELL, 36.0 + (n / 8) as f32 * CELL, CELL - 2.0, CELL - 2.0);
            draw_rectangle(r.x, r.y, r.w, r.h, col(1));
            match self.looks.get(&(base, k)).and_then(|v| v.as_ref()) {
                Some((tex, _, _)) => {
                    let f = ((r.w - 4.0) / tex.width()).min((r.h - 4.0) / tex.height()).min(2.0);
                    let (tw, th) = (tex.width() * f, tex.height() * f);
                    draw_texture_ex(tex, r.x + (r.w - tw) / 2.0, r.y + (r.h - th) / 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(tw, th)), ..Default::default() });
                }
                None => {
                    draw_text(&format!("{k:02X}"), r.x + 6.0, r.y + 22.0, 16.0, col(3));
                }
            }
        }
        self.status_line_blank();
        if create {
            self.blank_choice = None;
            self.make_blank_level(base);
        } else {
            self.blank_choice = choice;
            if choice.is_none() {
                self.status = tr(es, "No level made.", "No se ha creado ningun nivel.").into();
            }
        }
    }

    fn status_line_blank(&self) {
        let h = 22.0;
        draw_rectangle(self.panel, screen_height() - h, screen_width() - self.panel, h, col(1));
        draw_text(&fit(&self.status, screen_width() - self.panel - 16.0, 16.0), self.panel + 8.0, screen_height() - 6.0, 16.0, col(3));
    }

    /// What else comes with game level `base` besides its pictures.
    fn blank_notes(&self, base: usize) -> Vec<String> {
        let es = self.es;
        let t = Tables::read(&self.doc.original, base);
        let mut v = vec![format!(
            "{} {base:02}: {}",
            tr(es, "Its tiles, objects, music and time are level", "Sus tiles, objetos, musica y tiempo son los del nivel"),
            tr(es, "an empty sky, a floor and Mario's start.", "un cielo vacio, un suelo y el inicio de Mario.")
        )];
        if t.space_gravity() {
            v.push(tr(es, "It has space gravity (its settings can change it).", "Tiene gravedad de espacio (sus ajustes la cambian).").into());
        }
        if t.wide_camera() {
            v.push(tr(es, "Its camera has the wide margin (its settings can change it).", "Su camara tiene el margen amplio (sus ajustes lo cambian).").into());
        }
        if t.animated() {
            v.push(tr(es, "Some of its tiles are animated (its settings can stop them).", "Algunos de sus tiles se animan (sus ajustes los paran).").into());
        }
        if base == WARIO_ROOM {
            v.push(tr(es, "Level 24 is Wario's final room: no map point opens it.", "El nivel 24 es la sala final de Wario: ningun punto del mapa lo abre.").into());
        }
        v.push(format!("{}: {}", tr(es, "Key", "Tecla"), keys(Action::NewBlankLevel)));
        v
    }

    fn make_blank_level(&mut self, base: usize) {
        match self.doc.add_blank_level(base) {
            Some(n) => {
                self.select_level(n);
                self.status = format!(
                    "{} {n:02}: {} {base:02}. {}",
                    tr(self.es, "New blank level", "Nivel en blanco nuevo"),
                    tr(self.es, "plays as level", "se juega como el nivel"),
                    tr(self.es, "Give it to a map point to reach it in the game.", "Asignalo a un punto del mapa para llegar a el en el juego.")
                );
            }
            None => self.status = tr(self.es, "There can't be more levels.", "No caben mas niveles.").into(),
        }
    }
}
