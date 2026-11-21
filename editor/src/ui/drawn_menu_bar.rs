//! The menu bar drawn along the top of the window, where the system gives none (Linux): the same
//! menus, shortcuts, greyed-out and ticked entries as the Windows bar. A click on a title opens
//! its menu; with one open, the pointer over another title opens that one; a click on an entry
//! chooses it, anywhere else (or Esc) closes the menu, and that click goes no further.
use super::widgets::{col, put_text, text_size, with_text_language, Lang};
use crate::ports::{MenuBar, MenuEntry, Taken};
use macroquad::prelude::*;

pub const BAR_H: f32 = 24.0;
const TEXT: f32 = 16.0;
const PAD: f32 = 10.0;
const ROW_H: f32 = 22.0;
const SEPARATOR_H: f32 = 9.0;
/// Room for the tick on the left of the entries.
const TICK_W: f32 = 22.0;
/// Between an entry's label and its shortcut.
const KEYS_GAP: f32 = 32.0;

#[derive(Clone, Debug, PartialEq)]
enum Entry {
    Item { id: u16, label: String, keys: String, enabled: bool, checked: bool },
    Separator,
}

#[derive(Clone, Debug)]
struct Menu {
    title: String,
    entries: Vec<Entry>,
}

/// The bar's menus and which is open: what it does with the pointer and Esc, apart from
/// drawing, so it can be checked without a window. `width` measures a text in pixels.
#[derive(Default)]
struct Bar {
    menus: Vec<Menu>,
    open: Option<usize>,
    chosen: Vec<u16>,
}

/// One frame's pointer and keys.
#[derive(Clone, Copy, Default)]
struct Input {
    pos: Vec2,
    click: bool,
    esc: bool,
}

/// A title as shown: Win32's `&` access-key marks dropped (`&&` is an `&`).
fn shown(title: &str) -> String {
    title.replace("&&", "\0").replace('&', "").replace('\0', "&")
}

impl Bar {
    /// Each title's place on the bar.
    fn titles(&self, width: &dyn Fn(&str) -> f32) -> Vec<Rect> {
        let mut x = 4.0;
        self.menus
            .iter()
            .map(|m| {
                let w = width(&shown(&m.title)) + 2.0 * PAD;
                let r = Rect::new(x, 0.0, w, BAR_H);
                x += w;
                r
            })
            .collect()
    }

    /// Menu `m`'s box below its title, and each entry's row in it.
    fn dropdown(&self, m: usize, width: &dyn Fn(&str) -> f32) -> (Rect, Vec<Rect>) {
        let title = self.titles(width)[m];
        let entries = &self.menus[m].entries;
        let inner = entries
            .iter()
            .map(|e| match e {
                Entry::Item { label, keys, .. } => width(label) + if keys.is_empty() { 0.0 } else { KEYS_GAP + width(keys) },
                Entry::Separator => 0.0,
            })
            .fold(0.0, f32::max);
        let w = (TICK_W + inner + 2.0 * PAD).max(title.w);
        let mut y = BAR_H + 2.0;
        let rows = entries
            .iter()
            .map(|e| {
                let h = if *e == Entry::Separator { SEPARATOR_H } else { ROW_H };
                let r = Rect::new(title.x + 1.0, y, w - 2.0, h);
                y += h;
                r
            })
            .collect();
        (Rect::new(title.x, BAR_H, w, y - BAR_H + 2.0), rows)
    }

    fn step(&mut self, i: Input, width: &dyn Fn(&str) -> f32) -> Taken {
        let was_open = self.open.is_some();
        let over_title = self.titles(width).iter().position(|r| r.contains(i.pos));
        if i.esc && was_open {
            self.open = None;
        } else if i.click {
            match (self.open, over_title) {
                (Some(m), Some(t)) if m == t => self.open = None,
                (_, Some(t)) => self.open = Some(t),
                (Some(m), None) => {
                    let (area, rows) = self.dropdown(m, width);
                    if area.contains(i.pos) {
                        if let Some(Entry::Item { id, enabled: true, .. }) = rows.iter().position(|r| r.contains(i.pos)).map(|k| &self.menus[m].entries[k]) {
                            self.chosen.push(*id);
                            self.open = None;
                        }
                    } else {
                        self.open = None;
                    }
                }
                (None, None) => {}
            }
        } else if let (Some(_), Some(t)) = (self.open, over_title) {
            self.open = Some(t);
        }
        let menu_shown = was_open || self.open.is_some();
        Taken { mouse: menu_shown || i.pos.y < BAR_H, keys: menu_shown }
    }

    fn entry_mut(&mut self, id: u16) -> impl Iterator<Item = (&mut bool, &mut bool)> {
        self.menus.iter_mut().flat_map(|m| m.entries.iter_mut()).filter_map(move |e| match e {
            Entry::Item { id: i, enabled, checked, .. } if *i == id => Some((enabled, checked)),
            _ => None,
        })
    }
}

/// The bar drawn in the window, in the editor's colours and language.
#[derive(Default)]
pub struct DrawnMenuBar {
    bar: Bar,
}

fn width(t: &str) -> f32 {
    with_own_font(t, || text_size(t, TEXT as u16).width)
}

/// A language's name (the Language menu) in that language's font, as the system shows it.
fn with_own_font<R>(t: &str, f: impl FnOnce() -> R) -> R {
    match Lang::ALL.iter().find(|l| l.name() == t) {
        Some(own) => with_text_language(*own, f),
        None => f(),
    }
}

impl MenuBar for DrawnMenuBar {
    fn add_menu(&mut self, title: &str, entries: &[MenuEntry]) {
        let entries = entries
            .iter()
            .map(|e| match e {
                MenuEntry::Item { id, label, keys } => Entry::Item { id: *id, label: label.to_string(), keys: keys.to_string(), enabled: true, checked: false },
                MenuEntry::Separator => Entry::Separator,
            })
            .collect();
        self.bar.menus.push(Menu { title: title.to_string(), entries });
    }

    fn chosen(&mut self) -> Vec<u16> {
        std::mem::take(&mut self.bar.chosen)
    }

    fn enable(&mut self, id: u16, on: bool) {
        self.bar.entry_mut(id).for_each(|(enabled, _)| *enabled = on);
    }

    fn check(&mut self, id: u16, on: bool) {
        self.bar.entry_mut(id).for_each(|(_, checked)| *checked = on);
    }

    fn clear(&mut self) {
        self.bar = Bar { chosen: std::mem::take(&mut self.bar.chosen), ..Bar::default() };
    }

    fn height(&self) -> f32 {
        BAR_H
    }

    fn input(&mut self) -> Taken {
        let (x, y) = mouse_position();
        let input = Input { pos: vec2(x, y), click: is_mouse_button_pressed(MouseButton::Left), esc: is_key_pressed(KeyCode::Escape) };
        self.bar.step(input, &width)
    }

    fn draw(&mut self) {
        let base = |r: Rect| r.y + (r.h + TEXT) / 2.0 - 3.0;
        let mouse: Vec2 = mouse_position().into();
        draw_rectangle(0.0, 0.0, screen_width(), BAR_H, col(0));
        draw_line(0.0, BAR_H - 0.5, screen_width(), BAR_H - 0.5, 1.0, col(1));
        for (m, r) in self.bar.titles(&width).iter().enumerate() {
            if self.bar.open == Some(m) || (self.bar.open.is_none() && r.contains(mouse)) {
                draw_rectangle(r.x, r.y + 1.0, r.w, r.h - 2.0, col(1));
            }
            put_text(&shown(&self.bar.menus[m].title), r.x + PAD, base(*r), TEXT, col(3));
        }
        let Some(m) = self.bar.open else { return };
        let (area, rows) = self.bar.dropdown(m, &width);
        draw_rectangle(area.x + 3.0, area.y + 3.0, area.w, area.h, Color::new(0.0, 0.0, 0.0, 0.25));
        draw_rectangle(area.x, area.y, area.w, area.h, col(0));
        draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, col(2));
        for (e, r) in self.bar.menus[m].entries.iter().zip(&rows) {
            let Entry::Item { label, keys, enabled, checked, .. } = e else {
                draw_line(r.x + 6.0, r.y + r.h / 2.0, r.x + r.w - 6.0, r.y + r.h / 2.0, 1.0, col(1));
                continue;
            };
            if *enabled && r.contains(mouse) {
                draw_rectangle(r.x, r.y, r.w, r.h, col(2));
            }
            let c = match (*enabled, r.contains(mouse)) {
                (false, _) => col(1),
                (true, true) => col(0),
                (true, false) => col(3),
            };
            if *checked {
                let (x, y) = (r.x + 7.0, r.y + r.h / 2.0);
                draw_line(x, y, x + 3.5, y + 4.0, 2.0, c);
                draw_line(x + 3.5, y + 4.0, x + 10.0, y - 4.0, 2.0, c);
            }
            with_own_font(label, || put_text(label, r.x + TICK_W, base(*r), TEXT, c));
            if !keys.is_empty() {
                put_text(keys, r.x + r.w - PAD - width(keys), base(*r), TEXT, c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 8 pixels a character: File at x 4..56, Tools at 56..116.
    fn width(t: &str) -> f32 {
        t.chars().count() as f32 * 8.0
    }

    fn bar() -> Bar {
        let item = |id, label: &str, enabled| Entry::Item { id, label: label.into(), keys: String::new(), enabled, checked: false };
        Bar {
            menus: vec![
                Menu { title: "&File".into(), entries: vec![item(1, "Save", true), Entry::Separator, item(2, "Export level...", false), item(3, "Export project...", true)] },
                Menu { title: "&Tools".into(), entries: vec![item(101, "Object catalogue", true)] },
            ],
            ..Bar::default()
        }
    }

    fn click(x: f32, y: f32) -> Input {
        Input { pos: vec2(x, y), click: true, esc: false }
    }

    fn hover(x: f32, y: f32) -> Input {
        Input { pos: vec2(x, y), ..Input::default() }
    }

    /// The middle of entry `k` of the File menu.
    fn file_entry(b: &Bar, k: usize) -> Input {
        let (_, rows) = b.dropdown(0, &width);
        click(rows[k].center().x, rows[k].center().y)
    }

    #[test]
    fn titles_drop_their_access_marks() {
        assert_eq!((shown("&File"), shown("Save && quit")), ("File".to_string(), "Save & quit".to_string()));
        assert_eq!(bar().titles(&width)[1].x, 4.0 + 4.0 * 8.0 + 2.0 * PAD);
    }

    #[test]
    fn a_click_on_a_title_opens_and_closes_its_menu() {
        let mut b = bar();
        assert_eq!(b.step(click(10.0, 10.0), &width), Taken { mouse: true, keys: true });
        assert_eq!(b.open, Some(0));
        b.step(click(10.0, 10.0), &width);
        assert_eq!(b.open, None);
    }

    #[test]
    fn hovering_another_title_opens_it_while_one_is_open() {
        let mut b = bar();
        b.step(hover(70.0, 10.0), &width);
        assert_eq!(b.open, None);
        b.step(click(10.0, 10.0), &width);
        b.step(hover(70.0, 10.0), &width);
        assert_eq!(b.open, Some(1));
    }

    #[test]
    fn choosing_an_entry() {
        let mut b = bar();
        b.step(click(10.0, 10.0), &width);
        let i = file_entry(&b, 3);
        b.step(i, &width);
        assert_eq!((b.open, b.chosen.clone()), (None, vec![3]));
    }

    #[test]
    fn greyed_out_entries_and_separators_do_nothing() {
        let mut b = bar();
        b.step(click(10.0, 10.0), &width);
        let (grey, sep) = (file_entry(&b, 2), file_entry(&b, 1));
        b.step(grey, &width);
        b.step(sep, &width);
        assert_eq!((b.open, b.chosen.len()), (Some(0), 0));
    }

    #[test]
    fn a_click_outside_or_esc_closes_without_passing_it_on() {
        let mut b = bar();
        b.step(click(10.0, 10.0), &width);
        assert_eq!(b.step(click(600.0, 400.0), &width), Taken { mouse: true, keys: true });
        assert_eq!((b.open, b.chosen.len()), (None, 0));
        b.step(click(10.0, 10.0), &width);
        assert_eq!(b.step(Input { esc: true, ..hover(600.0, 400.0) }, &width), Taken { mouse: true, keys: true });
        assert_eq!(b.open, None);
        assert_eq!(b.step(hover(600.0, 400.0), &width), Taken::default());
    }

    #[test]
    fn the_pointer_over_the_bar_is_the_bar_s() {
        assert_eq!(bar().step(hover(300.0, 5.0), &width), Taken { mouse: true, keys: false });
    }
}
