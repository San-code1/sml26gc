//! New blank level: one of the game's 32 levels is chosen while only its block palette and its
//! object palette are shown, as they look in that level; Create adds an empty level that plays
//! as it (`EditorDocument::add_blank_level`).
use super::palette::source;
use super::shortcuts::{keys, Action};
use super::widgets::*;
use super::Editor;
use cart_model::level_tables::Tables;
use cart_model::levels::LEVELS;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, is_key_pressed};

const PAD: f32 = 10.0;
const ROW: f32 = 26.0;
const LEVEL_BUTTON: (f32, f32) = (33.0, 23.0);
/// Wario's final room: no map point opens it.
const WARIO_ROOM: usize = 24;
const CELL: f32 = 36.0;

impl Editor {
    pub(super) fn open_blank_chooser(&mut self) {
        self.blank_choice = Some(self.doc.base_of(self.cur).min(LEVELS - 1));
        self.status = tr(self.lang, "New blank level: choose the level it plays as. Esc: cancel.").into();
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

    /// The chooser open on game level `level`, from the catalogue.
    pub(super) fn show_blank_chooser_at(&mut self, level: usize) {
        self.open_blank_chooser();
        self.blank_choice = Some(level.min(LEVELS - 1));
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
        let lang = self.lang;
        let w = self.panel - 2.0 * PAD - 8.0;
        put_text(tr(lang, "NEW BLANK LEVEL"), PAD, 25.0, 20.0, col(3));
        put_text(tr(lang, "Plays as:"), PAD, 52.0, 15.0, INFO);
        let mut y = 60.0;
        let per_row = ((w + 2.0) / (LEVEL_BUTTON.0 + 2.0)).floor().max(1.0) as usize;
        for l in 0..LEVELS {
            let r = Rect::new(PAD + (l % per_row) as f32 * (LEVEL_BUTTON.0 + 2.0), y + (l / per_row) as f32 * (LEVEL_BUTTON.1 + 2.0), LEVEL_BUTTON.0, LEVEL_BUTTON.1);
            if button(r, &format!("{l:02}"), l == base, 15.0) {
                choice = Some(l);
            }
        }
        y += LEVELS.div_ceil(per_row) as f32 * (LEVEL_BUTTON.1 + 2.0) + 10.0;
        let mut notes = self.blank_notes(base);
        if self.pictures_pending(base) {
            notes.push(tr(lang, "Some object pictures are still being captured; they show as numbers until then.").into());
        }
        for line in notes.iter().flat_map(|l| wrap(l, 34)) {
            put_text(&line, PAD, y + 12.0, 15.0, col(3));
            y += 17.0;
        }
        y += 10.0;
        let half = (w - 4.0) / 2.0;
        if button(Rect::new(PAD, y, half, ROW), tr(lang, "Create"), false, 15.0) || is_key_pressed(KeyCode::Enter) {
            create = true;
        }
        if button(Rect::new(PAD + half + 4.0, y, half, ROW), tr(lang, "Cancel (Esc)"), false, 15.0) || is_key_pressed(KeyCode::Escape) {
            choice = None;
        }
        // the view: the chosen level's blocks and objects
        let x0 = self.panel + 16.0;
        put_text(&format!("{} {base:02}", tr(lang, "BLOCKS OF LEVEL")), x0, 26.0, 18.0, col(0));
        if let Some(atlas) = self.atlases[base].clone() {
            for i in 0..128 {
                let r = Rect::new(x0 + (i % 8) as f32 * CELL, 36.0 + (i / 8) as f32 * CELL, CELL - 2.0, CELL - 2.0);
                draw_texture_ex(&atlas, r.x, r.y, WHITE, DrawTextureParams { source: Some(source(i, 16.0)), dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
            }
        }
        let x1 = x0 + 8.0 * CELL + 24.0;
        put_text(&format!("{} {base:02}", tr(lang, "OBJECTS OF LEVEL")), x1, 26.0, 18.0, col(0));
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
                    put_text(&format!("{k:02X}"), r.x + 6.0, r.y + 22.0, 16.0, col(3));
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
                self.status = tr(lang, "No level made.").into();
            }
        }
    }

    fn status_line_blank(&self) {
        let h = 22.0;
        draw_rectangle(self.panel, screen_height() - h, screen_width() - self.panel, h, col(1));
        status_message(&self.status, self.panel, screen_width(), screen_height() - h, h, 16.0, self.severity());
        self.capture_badge(screen_height() - h);
    }

    /// What else comes with game level `base` besides its pictures.
    fn blank_notes(&self, base: usize) -> Vec<String> {
        let lang = self.lang;
        let t = Tables::read(&self.doc.original, base);
        let mut v = vec![format!(
            "{} {base:02}: {}",
            tr(lang, "Its tiles, objects, music and time are level"),
            tr(lang, "an empty sky, a floor and Mario's start.")
        )];
        if t.space_gravity() {
            v.push(tr(lang, "It has space gravity (its settings can change it).").into());
        }
        if t.wide_camera() {
            v.push(tr(lang, "Its camera has the wide margin (its settings can change it).").into());
        }
        if t.animated() {
            v.push(tr(lang, "Some of its tiles are animated (its settings can stop them).").into());
        }
        if base == WARIO_ROOM {
            v.push(tr(lang, "Level 24 is Wario's final room: no map point opens it.").into());
        }
        v.push(format!("{}: {}", tr(lang, "Key"), keys(Action::NewBlankLevel)));
        v
    }

    fn make_blank_level(&mut self, base: usize) {
        match self.doc.add_blank_level(base) {
            Some(n) => {
                self.select_level(n);
                self.status = format!(
                    "{} {n:02}: {} {base:02}. {}",
                    tr(self.lang, "New blank level"),
                    tr(self.lang, "plays as level"),
                    tr(self.lang, "Give it to a map point to reach it in the game.")
                );
                self.mark(Severity::Done);
            }
            None => {
                self.status = tr(self.lang, "There can't be more levels.").into();
                self.mark(Severity::Warning);
            }
        }
    }
}
