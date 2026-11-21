//! Searching the object palette: a field under its category filter; typed words keep the types
//! whose name (in the editor's language) holds them, or whose number they are ("1b", "#1B").
//! While it is typed in it takes the keyboard; Enter or a click elsewhere keeps the text, Esc
//! clears it.
use super::widgets::*;
use super::Editor;
use macroquad::prelude::*;
use super::inset::{mouse_position, is_mouse_button_pressed, is_key_pressed, get_char_pressed};

const MOST: usize = 24;

#[derive(Default)]
pub(super) struct Search {
    text: String,
    typing: bool,
    field: Rect,
}

impl Editor {
    /// The search field at `y`, `w` wide; returns where the palette continues.
    pub(super) fn search_field(&mut self, y: f32, w: f32) -> f32 {
        let lang = self.lang;
        let label = tr(lang, "Search");
        let lw = text_size(label, 15).width.ceil() + 10.0;
        put_text(label, 10.0, y + 16.0, 15.0, col(3));
        let field = Rect::new(10.0 + lw, y, w - lw - 28.0, 22.0);
        self.search.field = field;
        if self.search.typing {
            draw_rectangle(field.x, field.y, field.w, field.h, col(3));
            let caret = if (get_time() * 2.0) as i64 % 2 == 0 { "_" } else { "" };
            put_text(&format!("{}{caret}", self.search.text), field.x + 6.0, field.y + 16.0, 15.0, col(0));
        } else {
            let (text, c) = if self.search.text.is_empty() { (tr(lang, "name or number").to_string(), col(2)) } else { (self.search.text.clone(), col(3)) };
            draw_rectangle_lines(field.x, field.y, field.w, field.h, 1.0, col(2));
            put_text(&fit(&text, field.w - 12.0, 15.0), field.x + 6.0, field.y + 16.0, 15.0, c);
            if field.contains(mouse_position().into()) && is_mouse_button_pressed(MouseButton::Left) {
                while get_char_pressed().is_some() {}
                self.search.typing = true;
            }
        }
        if button(Rect::new(field.x + field.w + 6.0, y, 22.0, 22.0), "x", false, 15.0) {
            self.search = Search::default();
        }
        y + 28.0
    }

    /// Typing into the search field. Returns whether the keyboard was taken this frame.
    pub(super) fn update_search(&mut self) -> bool {
        if !self.search.typing {
            return false;
        }
        while let Some(c) = get_char_pressed() {
            if !c.is_control() && self.search.text.chars().count() < MOST {
                self.search.text.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.search.text.pop();
        }
        let outside = is_mouse_button_pressed(MouseButton::Left) && !self.search.field.contains(mouse_position().into());
        if is_key_pressed(KeyCode::Escape) {
            self.search = Search::default();
        } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || outside {
            self.search.typing = false;
        }
        true
    }

    /// Does object type `k` of the level shown match the search (always, with none)?
    pub(super) fn search_matches(&mut self, k: u8) -> bool {
        let q = self.search.text.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let number = q.trim_start_matches('#').trim_start_matches('$');
        if u8::from_str_radix(number, 16).is_ok_and(|n| n == k) && number.len() <= 2 {
            return true;
        }
        self.object_label(self.look_key(k)).is_some_and(|(name, _)| name.to_lowercase().contains(&q))
    }
}
