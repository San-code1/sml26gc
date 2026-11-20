//! The level editor's tools for trying levels: checkpoints (midway bells and where Mario
//! starts again after ringing one), the level's settings (start time, music, background
//! shades), its warnings, and the marks drawn over the level for them.
use super::start_view::Placing;
use super::widgets::*;
use super::Editor;
use crate::domain::document::check::Warning;
use crate::domain::document::CheckpointRefused;
use cart_model::checkpoint;
use cart_model::level_start::{SPRITE_H, SPRITE_TOP, SPRITE_W};
use macroquad::prelude::*;

pub(super) fn warn_col() -> Color {
    Color::new(1.0, 0.45, 0.35, 1.0)
}

impl Editor {
    /// A click on the level in "Play here": the game starts there.
    pub(super) fn update_play_here(&mut self, m: Vec2) -> bool {
        if self.placing != Some(Placing::PlayHere) || !is_mouse_button_pressed(MouseButton::Left) || self.cell_at(m).is_none() {
            return false;
        }
        self.play_from_here(self.world_pos(m));
        true
    }

    /// In "Play here", Mario's outline where a click would start him, and the band whose
    /// objects would not appear then.
    pub(super) fn draw_play_here(&self) {
        let m: Vec2 = mouse_position().into();
        if self.placing != Some(Placing::PlayHere) || self.cell_at(m).is_none() {
            return;
        }
        let k = self.scale();
        let s = Self::start_at(self.world_pos(m));
        self.draw_passed_band(&s, true);
        let ok = !s.blocked(&self.doc.levels[self.cur].content.map);
        let c = if ok { Color::new(1.0, 0.85, 0.1, 1.0) } else { warn_col() };
        // Mario as the game draws him there (his outline until his picture is captured)
        let (x, y) = match self.mario.as_ref().and_then(|m| m.as_ref()) {
            Some((t, dx, dy)) => {
                let (x, y) = (self.panel + (s.mario_x as f32 + dx - self.cam.x) * k, (s.mario_y as f32 + dy - self.cam.y) * k);
                draw_texture_ex(t, x, y, WHITE, DrawTextureParams { dest_size: Some(vec2(t.width(), t.height()) * k), ..Default::default() });
                marker(Rect::new(x - 2.0, y - 2.0, t.width() * k + 4.0, t.height() * k + 4.0), 2.0, c);
                (x, y)
            }
            None => {
                let (sx, sy) = s.sprite_at();
                let (x, y) = (self.panel + (sx as f32 - self.cam.x) * k, (sy as f32 - self.cam.y) * k);
                marker(Rect::new(x, y, SPRITE_W as f32 * k, (SPRITE_H - SPRITE_TOP) as f32 * k), 2.0, c);
                (x, y)
            }
        };
        let t = tr(self.lang, "PLAY HERE");
        put_text(t, x + 1.0, y - 5.0, 16.0, BLACK);
        put_text(t, x, y - 6.0, 16.0, c);
    }

    /// A click on the level while placing a checkpoint. True when it took the click.
    pub(super) fn update_checkpoint(&mut self, m: Vec2) -> bool {
        if self.placing != Some(Placing::Checkpoint) || !is_mouse_button_pressed(MouseButton::Left) {
            return false;
        }
        let Some((c, r)) = self.cell_at(m) else { return false };
        let l = self.cur;
        if let Some(why) = self.doc.checkpoint_refused(l) {
            self.status = self.refused_text(why);
            self.mark(Severity::Warning);
            return true;
        }
        self.doc.snapshot(l);
        self.doc.add_bell(l, c as u16 * 16, r as u16 * 16);
        self.overview_stale = true;
        true
    }

    fn refused_text(&self, why: CheckpointRefused) -> String {
        match why {
            CheckpointRefused::NoRoom => tr(self.lang, "The game keeps no restart place for this level: it cannot have checkpoints.").into(),
            CheckpointRefused::Full => format!("{} {}.", tr(self.lang, "A level can have at most"), checkpoint::MAX),
        }
    }

    /// Each checkpoint bell named, and Mario's outline where he starts again after it.
    pub(super) fn draw_checkpoints(&self) {
        let k = self.scale();
        let c = &self.doc.levels[self.cur].content;
        let on = self.placing == Some(Placing::Checkpoint);
        let colour = if on { Color::new(1.0, 0.85, 0.1, 1.0) } else { Color::new(1.0, 0.6, 0.9, 0.9) };
        for (n, i) in checkpoint::bells(&c.objs).into_iter().enumerate() {
            let b = &c.objs[i];
            let (x, y) = (self.panel + (b.x as f32 - self.cam.x) * k, (b.y as f32 - self.cam.y) * k);
            let name = format!("{} {}", tr(self.lang, "CHECKPOINT"), n + 1);
            put_text(&name, x + 1.0, y - 5.0, 16.0, BLACK);
            put_text(&name, x, y - 6.0, 16.0, colour);
            let s = checkpoint::restart_for(&c.map, b);
            let (sx, sy) = s.sprite_at();
            let (rx, ry) = (self.panel + (sx as f32 - self.cam.x) * k, (sy as f32 - self.cam.y) * k);
            marker(Rect::new(rx, ry, SPRITE_W as f32 * k, (SPRITE_H - SPRITE_TOP) as f32 * k), 2.0, colour);
            put_text(&format!("{}", n + 1), rx + 3.0, ry + 14.0, 16.0, colour);
        }
        if let Some((p, until)) = self.mark {
            if get_time() < until && (get_time() * 6.0) as i64 % 2 == 0 {
                let (x, y) = (self.panel + (p.x - self.cam.x) * k, (p.y - self.cam.y) * k);
                marker(Rect::new(x - 8.0 * k, y - 8.0 * k, 32.0 * k, 32.0 * k), 3.0, warn_col());
            }
        }
    }

    pub(super) fn set_music_volume(&mut self, v: u32) {
        self.prefs.music_volume = v;
        self.prefs_store.save(&self.prefs);
    }

    pub(super) fn set_test_game(&mut self, lives: u32, coins: u32, power: u32) {
        (self.prefs.test_lives, self.prefs.test_coins, self.prefs.test_power) = (lives, coins, power);
        self.prefs_store.save(&self.prefs);
    }

    pub(super) fn warning_text(&self, w: &Warning) -> String {
        let lang = self.lang;
        match *w {
            Warning::NoFinish => tr(lang, "No finish").into(),
            Warning::FinishWithoutBell(_) => tr(lang, "Finish without its bell").into(),
            Warning::BellWithoutFinish(_) => tr(lang, "Bell with no finish below").into(),
            Warning::CrowdedScreen(_, n) => format!("{n} {}", tr(lang, "objects in one screen (game: 6 at most)")),
            Warning::TooManyObjects(n) => format!("{n} {}", tr(lang, "objects (the game holds 83)")),
            Warning::StartBlocked(_) => tr(lang, "Start inside blocks").into(),
            Warning::RestartBlocked(_) => tr(lang, "Checkpoint restart inside blocks").into(),
            Warning::MissingLevel(p, l) => format!("{} {p:02X} {} {l:02}", tr(lang, "Point"), tr(lang, "opens missing level")),
            Warning::OnFirstScreen(_, n) => format!("{n} {}", tr(lang, "objects that do not appear at the start")),
            Warning::IdleBlockContent(_) => tr(lang, "Hidden object over a block that does not release it").into(),
        }
    }
}
