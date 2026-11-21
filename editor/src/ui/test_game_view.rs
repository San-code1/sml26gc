//! The test game's settings: the lives, coins and power Play and Here start with (kept in
//! editor.cfg). The button beside Play drops them as a small window over the view, the same in
//! the level and zone editors; a click outside or Esc closes it.
use super::level_panel::{GAP, ROW};
use super::Editor;
use super::widgets::*;
use crate::domain::prefs::{Prefs, TEST_COINS, TEST_LIVES, TEST_POWERS};
use macroquad::prelude::*;
use super::inset::{screen_height, mouse_position, is_mouse_button_pressed, is_key_pressed};

const W: f32 = 300.0;
const PAD: f32 = 12.0;
const LABEL_W: f32 = 90.0;
const NOTE: f32 = 14.0;
/// Mario's powers, in `TEST_POWERS` order.
const POWERS: [&str; 4] = ["Small", "Big", "Carrot", "Fire"];

impl Editor {
    /// The button beside Play: it opens and closes the settings, its tooltip says them.
    pub(super) fn test_game_button(&mut self, r: Rect) {
        self.test_game_anchor = r;
        let lang = self.lang;
        if r.contains(mouse_position().into()) {
            let power = tr(lang, POWERS[self.prefs.test_power as usize % POWERS.len()]);
            self.tip = Some(format!("{}: {} {}, {} {}, Mario {power}", tr(lang, "Test game"), self.prefs.test_lives, tr(lang, "lives"), self.prefs.test_coins, tr(lang, "coins")));
        }
        if button(r, if self.test_game_open { "^" } else { "v" }, self.test_game_open, 15.0) {
            self.test_game_open = !self.test_game_open;
        }
    }

    /// The settings open (for the check pictures).
    pub fn show_test_game(&mut self) {
        self.test_game_open = true;
    }

    /// The settings window, when open, over everything but the tooltip.
    pub(super) fn test_game_popover(&mut self) {
        if !self.test_game_open {
            return;
        }
        let lang = self.lang;
        let inner = W - 2.0 * PAD;
        let note = wrap_px(tr(lang, "Play and Here start a fresh game with these and keep no saves. Shift: 10 lives or 100 coins at a time."), inner, NOTE);
        let h = PAD + 18.0 + 3.0 * (ROW + GAP) + 4.0 + note.len() as f32 * (NOTE + 2.0) + 8.0 + ROW + PAD;
        let a = self.test_game_anchor;
        let x = self.panel + 8.0;
        let y = a.y.min(screen_height() - STATUS_H - h - 4.0).max(4.0);
        let r = Rect::new(x, y, W, h);
        let m: Vec2 = mouse_position().into();
        if is_key_pressed(KeyCode::Escape) || (is_mouse_button_pressed(MouseButton::Left) && !r.contains(m) && !a.contains(m)) {
            self.test_game_open = false;
            return;
        }
        draw_rectangle(r.x + 4.0, r.y + 4.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_rectangle(r.x, r.y, r.w, r.h, col(0));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col(3));
        // the title, joined to the button it drops from
        let mut cy = y + PAD;
        let title = tr(lang, "TEST GAME");
        put_text(title, x + PAD, cy + 13.0, 14.0, col(2));
        let d = text_size(title, 14).width.ceil();
        draw_rectangle(x + PAD + d + 8.0, cy + 8.0, inner - d - 8.0, 1.0, col(1));
        cy += 18.0 + GAP;
        let far = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) { 10 } else { 1 };
        let (mut lives, mut coins) = (self.prefs.test_lives as i32, self.prefs.test_coins as i32);
        lives += far * stepper(x + PAD, cy, inner, tr(lang, "Lives"), &lives.to_string());
        cy += ROW + GAP;
        coins += 10 * far * stepper(x + PAD, cy, inner, tr(lang, "Coins"), &coins.to_string());
        cy += ROW + GAP;
        let mut power = self.prefs.test_power as i32;
        power += stepper(x + PAD, cy, inner, "Mario", tr(lang, POWERS[power as usize % POWERS.len()]));
        cy += ROW + GAP + 4.0;
        for line in &note {
            put_text(line, x + PAD, cy + NOTE - 2.0, NOTE, INFO);
            cy += NOTE + 2.0;
        }
        cy += 8.0;
        let d = Prefs::default();
        let bw = 120.0;
        if button(Rect::new(x + W - PAD - bw, cy, bw, ROW), tr(lang, "Defaults"), false, 15.0) {
            (lives, coins, power) = (d.test_lives as i32, d.test_coins as i32, d.test_power as i32);
        }
        let lives = lives.clamp(*TEST_LIVES.start() as i32, *TEST_LIVES.end() as i32) as u32;
        let coins = coins.clamp(*TEST_COINS.start() as i32, *TEST_COINS.end() as i32) as u32;
        let power = power.rem_euclid(*TEST_POWERS.end() as i32 + 1) as u32;
        if (lives, coins, power) != (self.prefs.test_lives, self.prefs.test_coins, self.prefs.test_power) {
            self.set_test_game(lives, coins, power);
        }
    }
}

/// A name, then `-` value `+` over the rest of `w`; returns -1, 0 or 1.
fn stepper(x: f32, y: f32, w: f32, name: &str, value: &str) -> i32 {
    put_text(name, x, y + 18.0, 15.0, col(3));
    let (cx, cw) = (x + LABEL_W, w - LABEL_W);
    let mut d = 0;
    if button(Rect::new(cx, y, ROW, ROW), "-", false, 16.0) {
        d = -1;
    }
    let vw = cw - 2.0 * (ROW + GAP);
    let t = text_size(value, 16);
    put_text(value, (cx + ROW + GAP + (vw - t.width) / 2.0).round(), y + 18.0, 16.0, col(3));
    if button(Rect::new(cx + cw - ROW, y, ROW, ROW), "+", false, 16.0) {
        d = 1;
    }
    d
}
