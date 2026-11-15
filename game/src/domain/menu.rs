//! Pause menu drawn at Game Boy resolution in the screen's four shades: key bindings,
//! display, sound, language, the game's own pause, back to the map, file select, quit.
//! It changes `Settings` in place; saving them and switching full screen is the caller's job.
use super::i18n::{texts, Texts, LANG_NAMES};
use super::keys::{self, Key, DEFAULT_KEYS};
use super::overlay::{text_width, Canvas};
use super::settings::Settings;
use crate::ports::Keyboard;
use cart_model::GameMode;
use gb_core::ppu::H;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    Controls,
    Display,
    Sound,
    Language,
    Waiting(usize),
    /// Asking before leaving the level or the game: holds the main item.
    Confirm(usize),
}

pub enum Action {
    None,
    Close,
    PressStart,
    Quit,
    BackToMap,
    FileSelect,
}

// Main menu items (indexes into Texts::main).
const IT_CONTINUE: usize = 0;
const IT_CONTROLS: usize = 1;
const IT_DISPLAY: usize = 2;
const IT_SOUND: usize = 3;
const IT_LANGUAGE: usize = 4;
const IT_PAUSE: usize = 5;
const IT_QUIT: usize = 6;
const IT_MAP: usize = 7;
const IT_FILES: usize = 8;

pub struct Menu {
    screen: Screen,
    sel: usize,
    blink: u32,
    items: Vec<usize>,
}

impl Menu {
    fn label<'a>(&self, t: &'a Texts, item: usize) -> &'a str {
        t.main[item]
    }

    /// `mode` is the screen the game is on (a level, the map...).
    pub fn new(mode: GameMode) -> Menu {
        let in_level = mode == GameMode::Playing;
        let on_map = mode == GameMode::Map;
        // No "continue" item: Esc (or B) closes the menu. In a level the game's own pause comes first.
        let mut items = Vec::new();
        if in_level {
            items.push(IT_PAUSE);
        }
        items.extend([IT_CONTROLS, IT_DISPLAY, IT_SOUND, IT_LANGUAGE]);
        if in_level {
            items.push(IT_MAP);
        }
        if in_level || on_map {
            items.push(IT_FILES);
        }
        items.push(IT_QUIT);
        Menu { screen: Screen::Main, sel: 0, blink: 0, items }
    }

    fn pos(&self, item: usize) -> usize {
        self.items.iter().position(|&i| i == item).unwrap_or(0)
    }

    /// For previews: show one screen ("main", "controls", "wait", "display", "sound", "language").
    pub fn preview(name: &str) -> Menu {
        let screen = match name {
            "controls" => Screen::Controls,
            "wait" => Screen::Waiting(4),
            "display" => Screen::Display,
            "sound" => Screen::Sound,
            "language" => Screen::Language,
            "confirm" => Screen::Confirm(IT_MAP),
            _ => Screen::Main,
        };
        let mut m = Menu::new(GameMode::Playing);
        m.screen = screen;
        m
    }

    fn count(&self) -> usize {
        match self.screen {
            Screen::Main => self.items.len(),
            Screen::Confirm(_) => 2,
            Screen::Controls => 10,
            Screen::Display => 4,
            Screen::Sound => 2,
            Screen::Language => 4,
            Screen::Waiting(_) => 0,
        }
    }

    fn go(&mut self, s: Screen, sel: usize) {
        self.screen = s;
        self.sel = sel;
    }

    pub fn update(&mut self, input: &dyn Keyboard, st: &mut Settings) -> Action {
        self.blink = self.blink.wrapping_add(1);
        if let Screen::Waiting(b) = self.screen {
            if input.pressed(Key::Escape) {
                self.go(Screen::Controls, b);
                return Action::None;
            }
            if let Some(k) = input.last_pressed() {
                if k.bindable() {
                    // A key already used elsewhere swaps places, so no button is lost.
                    if let Some(other) = st.keys.iter().position(|&x| x == k) {
                        st.keys[other] = st.keys[b];
                    }
                    st.keys[b] = k;
                    self.go(Screen::Controls, b);
                }
            }
            return Action::None;
        }
        let any = |ks: &[Key]| ks.iter().any(|&k| input.pressed(k));
        let up = any(&[Key::Up, st.keys[keys::UP]]);
        let down = any(&[Key::Down, st.keys[keys::DOWN]]);
        let left = any(&[Key::Left, st.keys[keys::LEFT]]);
        let right = any(&[Key::Right, st.keys[keys::RIGHT]]);
        let ok = any(&[Key::Enter, Key::KpEnter, Key::Space, st.keys[keys::A]]);
        let back = any(&[Key::Escape, st.keys[keys::B]]);
        let n = self.count();
        // Yes/No sit side by side, so they are chosen with Left/Right; lists use Up/Down.
        let (prev, next) = if matches!(self.screen, Screen::Confirm(_)) { (left, right) } else { (up, down) };
        if prev {
            self.sel = (self.sel + n - 1) % n;
        }
        if next {
            self.sel = (self.sel + 1) % n;
        }
        if back {
            let back_to = match self.screen {
                Screen::Main => return Action::Close,
                Screen::Controls => self.pos(IT_CONTROLS),
                Screen::Display => self.pos(IT_DISPLAY),
                Screen::Sound => self.pos(IT_SOUND),
                Screen::Confirm(item) => self.pos(item),
                _ => self.pos(IT_LANGUAGE),
            };
            self.go(Screen::Main, back_to);
            return Action::None;
        }
        let step: i32 = if right { 1 } else if left { -1 } else { 0 };
        match self.screen {
            Screen::Main => {
                if ok {
                    match self.items[self.sel] {
                        IT_CONTINUE => return Action::Close,
                        IT_CONTROLS => self.go(Screen::Controls, 0),
                        IT_DISPLAY => self.go(Screen::Display, 0),
                        IT_SOUND => self.go(Screen::Sound, 0),
                        IT_LANGUAGE => self.go(Screen::Language, st.lang),
                        IT_PAUSE => return Action::PressStart,
                        // "No" is selected first, so a stray key press never leaves the level.
                        item => self.go(Screen::Confirm(item), 1),
                    }
                }
            }
            Screen::Confirm(item) => {
                if ok {
                    if self.sel == 0 {
                        return match item {
                            IT_MAP => Action::BackToMap,
                            IT_FILES => Action::FileSelect,
                            _ => Action::Quit,
                        };
                    }
                    self.go(Screen::Main, self.pos(item));
                }
            }
            Screen::Controls => {
                if ok {
                    match self.sel {
                        0..=7 => self.screen = Screen::Waiting(self.sel),
                        8 => {
                            st.keys = DEFAULT_KEYS;
                        }
                        _ => self.go(Screen::Main, self.pos(IT_CONTROLS)),
                    }
                }
            }
            Screen::Display => {
                let change = step != 0 || ok;
                match self.sel {
                    0 if change => {
                        st.palette = ((st.palette as i32 + if step == 0 { 1 } else { step } + 3) % 3) as usize;
                    }
                    1 if change => {
                        st.fullscreen = !st.fullscreen;
                    }
                    2 if change => {
                        st.integer_scale = !st.integer_scale;
                    }
                    3 if ok => self.go(Screen::Main, self.pos(IT_DISPLAY)),
                    _ => {}
                }
            }
            Screen::Sound => match self.sel {
                0 if step != 0 => {
                    st.volume = (st.volume as i32 + step).clamp(0, 10) as u8;
                }
                1 if ok => self.go(Screen::Main, self.pos(IT_SOUND)),
                _ => {}
            },
            Screen::Language => {
                if ok {
                    if self.sel < 3 {
                        st.lang = self.sel;
                    } else {
                        self.go(Screen::Main, self.pos(IT_LANGUAGE));
                    }
                }
            }
            Screen::Waiting(_) => {}
        }
        Action::None
    }

    pub fn draw(&self, px: &mut [u8], st: &Settings) {
        let t = texts(st.lang);
        let mut c = Canvas { px };
        c.veil();
        let blink_on = (self.blink / 16) % 2 == 0;
        let shade = |i: usize| if i == self.sel { 3 } else { 2 };
        match self.screen {
            Screen::Main => {
                let n = self.items.len() as i32;
                let gap = if n > 8 { 11 } else { 12 };
                let h = 28 + n * gap;
                let (x, w) = (14, 132);
                let y = ((H as i32 - h) / 2).max(6);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.pause);
                for (i, &item) in self.items.iter().enumerate() {
                    let ty = y + 12 + i as i32 * gap;
                    c.text(x + 18, ty, self.label(t, item), shade(i));
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_main);
            }
            Screen::Confirm(item) => {
                let (x, y, w, h) = (14, 40, 132, 64);
                c.frame(x, y, w, h);
                c.title(x, y, w, self.label(t, item));
                c.text(x + (w - text_width(t.sure)) / 2, y + 14, t.sure, 3);
                for (i, label) in [t.yes, t.no].iter().enumerate() {
                    let tx = x + 30 + i as i32 * 50;
                    c.text(tx, y + 32, label, shade(i));
                    if i == self.sel {
                        c.cursor(tx - 9, y + 32, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_language);
            }
            Screen::Controls => {
                let (x, y, w, h) = (2, 8, 156, 132);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.controls);
                for i in 0..10 {
                    let ty = y + 11 + i as i32 * 11;
                    if i < 8 {
                        c.text(x + 16, ty, t.buttons[i], shade(i));
                        let label = st.keys[i].label(st.lang);
                        c.text(x + w - 8 - text_width(label), ty, label, 3);
                    } else {
                        c.text(x + 16, ty, if i == 8 { t.restore } else { t.back }, shade(i));
                    }
                    if i == self.sel {
                        c.cursor(x + 8, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_controls);
            }
            Screen::Waiting(b) => {
                let (x, y, w, h) = (14, 44, 132, 56);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.buttons[b]);
                if blink_on {
                    c.text(x + (w - text_width(t.press_key)) / 2, y + 14, t.press_key, 3);
                }
                c.text(x + 18, y + 28, t.now, 2);
                c.text(x + 22 + text_width(t.now), y + 28, st.keys[b].label(st.lang), 3);
                c.text(x + (w - text_width(t.cancel)) / 2, y + 40, t.cancel, 2);
            }
            Screen::Display => {
                let (x, y, w, h) = (6, 30, 148, 84);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.display);
                let rows = [
                    (t.colours, t.palettes[st.palette % 3]),
                    (t.fullscreen, if st.fullscreen { t.yes } else { t.no }),
                    (t.scale, if st.integer_scale { t.sharp } else { t.fitted }),
                    (t.back, ""),
                ];
                for (i, (l, v)) in rows.iter().enumerate() {
                    let ty = y + 14 + i as i32 * 13;
                    c.text(x + 18, ty, l, shade(i));
                    if !v.is_empty() {
                        let vx = x + w - 14 - text_width(v);
                        c.text(vx, ty, v, 3);
                        if i == self.sel {
                            c.text(vx - 8, ty, "<", 2);
                            c.text(x + w - 12, ty, ">", 2);
                        }
                    }
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                    if i == 0 {
                        // palette swatch: the four shades of the chosen colours
                        let sx = x + 18 + text_width(l) + 4;
                        c.rect(sx, ty - 1, 18, 9, 3);
                        for k in 0..4u8 {
                            c.rect(sx + 1 + k as i32 * 4, ty, 4, 7, k);
                        }
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_change);
            }
            Screen::Sound => {
                let (x, y, w, h) = (14, 44, 132, 56);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.sound);
                c.text(x + 18, y + 13, t.volume, shade(0));
                let bx = x + 18;
                let by = y + 24;
                c.rect(bx - 1, by - 1, 10 * 9 + 1, 9, 3);
                c.rect(bx, by, 10 * 9 - 1, 7, 0);
                for i in 0..10 {
                    if i < st.volume as i32 {
                        c.rect(bx + 1 + i * 9, by + 1, 7, 5, 3);
                    } else {
                        c.rect(bx + 1 + i * 9, by + 3, 7, 1, 1);
                    }
                }
                c.text(bx + 94, by, &format!("{}", st.volume), 3);
                c.text(x + 18, y + 38, t.back, shade(1));
                c.cursor(x + 9, if self.sel == 0 { y + 13 } else { y + 38 }, 3);
            }
            Screen::Language => {
                let (x, y, w, h) = (22, 34, 116, 76);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.language);
                for i in 0..4 {
                    let ty = y + 13 + i as i32 * 12;
                    let label = if i < 3 { LANG_NAMES[i] } else { t.back };
                    c.text(x + 18, ty, label, shade(i));
                    if i < 3 && i == st.lang {
                        // the language in use is marked with a filled box, like a checked option
                        c.rect(x + w - 16, ty, 7, 7, 3);
                        c.rect(x + w - 15, ty + 1, 5, 5, 0);
                        c.rect(x + w - 14, ty + 2, 3, 3, 3);
                    } else if i < 3 {
                        c.rect(x + w - 16, ty, 7, 7, 3);
                        c.rect(x + w - 15, ty + 1, 5, 5, 0);
                    }
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, t.footer_language);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One key going down per frame.
    struct Press(Option<Key>);
    impl Keyboard for Press {
        fn pressed(&self, k: Key) -> bool {
            self.0 == Some(k)
        }
        fn held(&self, k: Key) -> bool {
            self.0 == Some(k)
        }
        fn last_pressed(&self) -> Option<Key> {
            self.0
        }
    }

    fn step(m: &mut Menu, st: &mut Settings, k: Key) -> Action {
        m.update(&Press(Some(k)), st)
    }

    #[test]
    fn change_palette_rebind_and_leave() {
        let mut st = Settings::default();
        let mut m = Menu::new(GameMode::Playing);
        // Original pause, Controls, Display...
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Right);
        assert_eq!(st.palette, 1);
        step(&mut m, &mut st, Key::Escape);
        // Controls: bind A to Space.
        step(&mut m, &mut st, Key::Up);
        step(&mut m, &mut st, Key::Enter);
        for _ in 0..4 {
            step(&mut m, &mut st, Key::Down);
        }
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Space);
        assert_eq!(st.keys[keys::A], Key::Space);
        step(&mut m, &mut st, Key::Escape);
        assert!(matches!(step(&mut m, &mut st, Key::Escape), Action::Close));
    }

    #[test]
    fn leaving_the_level_asks_first() {
        let mut st = Settings::default();
        let mut m = Menu::new(GameMode::Playing);
        for _ in 0..5 {
            step(&mut m, &mut st, Key::Down); // ... Language, Back to map
        }
        step(&mut m, &mut st, Key::Enter);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::None)); // "No" comes first
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Left);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::BackToMap));
    }
}
