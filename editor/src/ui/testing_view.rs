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

    /// In "Play here", Mario's outline where a click would start him.
    pub(super) fn draw_play_here(&self) {
        let m: Vec2 = mouse_position().into();
        if self.placing != Some(Placing::PlayHere) || self.cell_at(m).is_none() {
            return;
        }
        let k = self.scale();
        let s = Self::start_at(self.world_pos(m));
        let (sx, sy) = s.sprite_at();
        let (x, y) = (self.panel + (sx as f32 - self.cam.x) * k, (sy as f32 - self.cam.y) * k);
        let ok = !s.blocked(&self.doc.levels[self.cur].content.map);
        let c = if ok { Color::new(1.0, 0.85, 0.1, 1.0) } else { warn_col() };
        marker(Rect::new(x, y, SPRITE_W as f32 * k, (SPRITE_H - SPRITE_TOP) as f32 * k), 2.0, c);
        let t = tr(self.es, "PLAY HERE", "JUGAR AQUI");
        draw_text(t, x + 1.0, y - 5.0, 16.0, BLACK);
        draw_text(t, x, y - 6.0, 16.0, c);
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
            return true;
        }
        self.doc.snapshot(l);
        self.doc.add_bell(l, c as u16 * 16, r as u16 * 16);
        self.overview_stale = true;
        true
    }

    fn refused_text(&self, why: CheckpointRefused) -> String {
        match why {
            CheckpointRefused::NoRoom => tr(self.es, "The game keeps no restart place for this level: it cannot have checkpoints.", "El juego no guarda punto de reaparicion para este nivel: no puede tener puntos de control.").into(),
            CheckpointRefused::Full => format!("{} {}.", tr(self.es, "A level can have at most", "Un nivel puede tener como mucho"), checkpoint::MAX),
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
            let name = format!("{} {}", tr(self.es, "CHECKPOINT", "CONTROL"), n + 1);
            draw_text(&name, x + 1.0, y - 5.0, 16.0, BLACK);
            draw_text(&name, x, y - 6.0, 16.0, colour);
            let s = checkpoint::restart_for(&c.map, b);
            let (sx, sy) = s.sprite_at();
            let (rx, ry) = (self.panel + (sx as f32 - self.cam.x) * k, (sy as f32 - self.cam.y) * k);
            marker(Rect::new(rx, ry, SPRITE_W as f32 * k, (SPRITE_H - SPRITE_TOP) as f32 * k), 2.0, colour);
            draw_text(&format!("{}", n + 1), rx + 3.0, ry + 14.0, 16.0, colour);
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

    pub(super) fn warning_text(&self, w: &Warning) -> String {
        let es = self.es;
        match *w {
            Warning::NoFinish => tr(es, "No finish", "Sin meta").into(),
            Warning::FinishWithoutBell(_) => tr(es, "Finish without its bell", "Meta sin su campana").into(),
            Warning::BellWithoutFinish(_) => tr(es, "Bell with no finish below", "Campana sin meta debajo").into(),
            Warning::CrowdedScreen(_, n) => format!("{n} {}", tr(es, "objects in one screen (game: 6 at most)", "objetos en una pantalla (el juego: 6 como mucho)")),
            Warning::TooManyObjects(n) => format!("{n} {}", tr(es, "objects (the game holds 83)", "objetos (el juego admite 83)")),
            Warning::StartBlocked(_) => tr(es, "Start inside blocks", "Inicio dentro de bloques").into(),
            Warning::RestartBlocked(_) => tr(es, "Checkpoint restart inside blocks", "Reaparicion dentro de bloques").into(),
            Warning::MissingLevel(p, l) => format!("{} {p:02X} {} {l:02}", tr(es, "Point", "Punto"), tr(es, "opens missing level", "abre el nivel inexistente")),
        }
    }
}
