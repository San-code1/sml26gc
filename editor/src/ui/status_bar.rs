//! The status bar: the last message, and in the level view how full the level's zone (or
//! the object space) is.
use super::widgets::*;
use super::{Editor, Mode};
use macroquad::prelude::*;

impl Editor {
    /// While objects are being captured in the background, a box at the bottom right of the
    /// view (above the status bar at `bottom`) says how far it is.
    pub(super) fn capture_badge(&self, bottom: f32) {
        let ((pl, pt), (tl, tt)) = self.look_loader.progress();
        if pl == 0 && tl == 0 {
            return;
        }
        let es = self.es;
        let text = format!("{}: {} {}/{}  {} {}/{}", tr(es, "Capturing objects", "Capturando objetos"), tr(es, "pictures", "dibujos"), pt - pl, pt, tr(es, "enemy tiles", "tiles de enemigos"), tt - tl, tt);
        let dim = measure_text(&text, None, 15, 1.0);
        let (w, h) = (dim.width + 16.0, 30.0);
        let (x, y) = (screen_width() - w - 10.0, bottom - h - 6.0);
        draw_rectangle(x, y, w, h, col(0));
        draw_rectangle_lines(x, y, w, h, 2.0, col(3));
        draw_text(&text, x + 8.0, y + 15.0, 15.0, col(3));
        let done = (pt - pl + tt - tl) as f32 / (pt + tt).max(1) as f32;
        draw_rectangle(x + 8.0, y + 21.0, (w - 16.0) * done, 4.0, col(2));
    }

    /// Game level `base` has object types whose pictures are not captured yet.
    pub(super) fn pictures_pending(&self, base: usize) -> bool {
        (0..128u8).any(|k| !self.looks.contains_key(&(base, k)))
    }

    /// Some of the game's levels' own objects have no picture yet (the catalogue groups by
    /// them).
    pub(super) fn own_pictures_pending(&self) -> bool {
        self.look_loader.progress().0 .0 > 0 && (0..cart_model::levels::LEVELS).any(|l| cart_model::objects::read(&self.doc.original, l).0.iter().any(|o| !self.looks.contains_key(&(l, o.kind))))
    }

    pub(super) fn status_line(&self, cw: f32, ch: f32) {
        draw_rectangle(self.panel, ch, cw, STATUS_H, col(1));
        status_message(&self.status, self.panel, self.panel + cw, ch, STATUS_H, 18.0, self.severity());
    }

    pub(super) fn level_status_bar(&mut self, cw: f32, ch: f32) {
        draw_rectangle(self.panel, ch, cw, STATUS_H, col(1));
        // measuring is not free: four times a second is enough
        if get_time() - self.space_at > 0.25 {
            self.space_cache = self.doc.zone_space(self.cur);
            self.obj_space = self.doc.object_space(self.cur);
            self.space_at = get_time();
        }
        let (need, cap) = if self.mode == Mode::Objects { self.obj_space } else { self.space_cache };
        let bw = 220.0;
        let bx = screen_width() - bw - 10.0;
        let frac = (need as f32 / cap.max(1) as f32).min(1.0);
        draw_rectangle(bx, ch + 6.0, bw, 14.0, col(0));
        draw_rectangle(bx, ch + 6.0, bw * frac, 14.0, if need > cap { Color::from_rgba(170, 40, 30, 255) } else { col(2) });
        draw_rectangle_lines(bx, ch + 6.0, bw, 14.0, 2.0, col(3));
        let what = match (self.mode == Mode::Objects, self.cur >= cart_model::levels::LEVELS) {
            (true, _) => tr(self.es, "object space", "espacio objetos"),
            (false, false) => tr(self.es, "zone space", "espacio zona"),
            (false, true) => tr(self.es, "level space", "espacio nivel"),
        };
        let label = format!("{what} {need}/{cap}");
        let dim = measure_text(&label, None, 16, 1.0);
        draw_text(&label, bx - dim.width - 8.0, ch + 18.0, 16.0, col(3));
        // the message takes the bar's left, and grows upwards when it is longer
        status_message(&self.status, self.panel, bx - dim.width - 20.0, ch, STATUS_H, 18.0, self.severity());
        self.capture_badge(ch);
    }
}
