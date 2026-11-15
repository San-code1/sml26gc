//! The status bar: the last message, and in the level view how full the level's zone (or
//! the object space) is.
use super::widgets::*;
use super::{Editor, Mode};
use macroquad::prelude::*;

impl Editor {
    pub(super) fn status_line(&self, cw: f32, ch: f32) {
        draw_rectangle(PANEL, ch, cw, STATUS_H, col(1));
        draw_text(&self.status, PANEL + 8.0, ch + 18.0, 18.0, col(3));
    }

    pub(super) fn level_status_bar(&mut self, cw: f32, ch: f32) {
        self.status_line(cw, ch);
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
    }
}
