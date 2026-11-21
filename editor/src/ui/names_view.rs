//! Naming new levels and new zones: a field in the panel that takes the keyboard while it is
//! being typed in (Enter or a click elsewhere keeps the name, Esc leaves it as it was).
use super::widgets::*;
use super::Editor;
use cart_model::extra_levels::NAME_MAX;
use macroquad::prelude::*;
use super::inset::{mouse_position, is_mouse_button_pressed, is_key_pressed, get_char_pressed};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Named {
    Level(usize),
    Zone(usize),
}

/// The name being typed: what it names, the text so far and where its field is drawn.
pub(super) struct Naming {
    what: Named,
    text: String,
    field: Rect,
}

impl Editor {
    fn name_of(&self, what: Named) -> &str {
        match what {
            Named::Level(l) => self.doc.level_name(l),
            Named::Zone(k) => self.doc.new_zones.get(k).map_or("", |z| z.name.as_str()),
        }
    }

    /// The name field of `what` at `r`: its name, or a click to start typing one.
    pub(super) fn name_field(&mut self, r: Rect, what: Named) {
        put_text(tr(self.lang, "Name"), r.x, r.y + 17.0, 16.0, col(3));
        let field = Rect::new(r.x + 54.0, r.y, r.w - 54.0, r.h);
        if let Some(n) = self.naming.as_mut().filter(|n| n.what == what) {
            n.field = field;
            draw_rectangle(field.x, field.y, field.w, field.h, col(3));
            let caret = if (get_time() * 2.0) as i64 % 2 == 0 { "_" } else { "" };
            put_text(&format!("{}{caret}", n.text), field.x + 6.0, field.y + 17.0, 16.0, col(0));
            return;
        }
        let name = self.name_of(what).to_string();
        let label = if name.is_empty() { tr(self.lang, "(none: click to name it)").to_string() } else { name.clone() };
        if button(field, &label, false, 15.0) {
            self.finish_naming(true);
            while get_char_pressed().is_some() {}
            self.naming = Some(Naming { what, text: name, field });
        }
    }

    /// Typing into the name field. Returns whether the keyboard was taken this frame.
    pub(super) fn update_naming(&mut self) -> bool {
        let Some(n) = self.naming.as_mut() else { return false };
        while let Some(c) = get_char_pressed() {
            if c.is_ascii() && !c.is_ascii_control() && n.text.chars().count() < NAME_MAX {
                n.text.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            n.text.pop();
        }
        let outside = is_mouse_button_pressed(MouseButton::Left) && !n.field.contains(mouse_position().into());
        if is_key_pressed(KeyCode::Escape) {
            self.finish_naming(false);
        } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) || outside {
            self.finish_naming(true);
        }
        true
    }

    /// Stop typing the name, keeping it or not.
    pub(super) fn finish_naming(&mut self, keep: bool) {
        let Some(n) = self.naming.take() else { return };
        if !keep {
            return;
        }
        let text = n.text.trim();
        match n.what {
            Named::Level(l) => self.doc.rename_level(l, text),
            Named::Zone(k) => self.doc.rename_zone(k, text),
        }
    }
}
