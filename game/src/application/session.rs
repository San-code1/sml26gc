//! One play session: the emulated console plus everything the port adds around it (pause
//! menu, the editor's new levels, battery saves, the start-up hint). Each call to `frame`
//! is one frame of the window.
use super::preferences::Preferences;
use crate::domain::keys::{self, Key};
use crate::domain::menu::{Action, Menu};
use crate::domain::overlay::draw_hint;
use crate::domain::settings::Settings;
use crate::ports::{AudioOut, Keyboard, SaveStore};
use cart_model::extra_levels;
use cart_model::ram_map::{GAME_MODE, LEVEL_NUMBER};
use cart_model::{GameDriver, GameMode};
use gb_core::palette::{Palette, PALETTES};
use gb_core::{Buttons, Gb, FRAME_SECONDS};

#[derive(PartialEq, Debug)]
pub enum Flow {
    Continue,
    Quit,
}

/// Writes that undo a new level put in place of its base level.
type Undo = Vec<(usize, Vec<u8>)>;

/// The level to ask the game for, to play `level`: a new level (32 and up, see
/// `extra_levels`) is put in place of its base level in the console's ROM, and the writes
/// that undo it come back. None if the copy has no such level.
fn stand_in(g: &mut Gb, level: u8) -> Option<(u8, Option<Undo>)> {
    let Some(k) = (level as usize).checked_sub(extra_levels::FIRST) else { return Some((level, None)) };
    let e = extra_levels::read(g.cart().rom()).into_iter().nth(k)?;
    let writes = extra_levels::writes_for(g.cart().rom(), &e);
    Some((e.base, Some(extra_levels::apply(g.cart_mut().rom_mut(), &writes))))
}

/// After each frame: when the game is about to load a level numbered 32 or more (a map
/// point the editor set to open one of its new levels), it is given that level's base
/// level with the new one in place. Checked while it is only setting up (modes $16 and $0B,
/// before the header is read).
fn take_new_level(g: &mut Gb, extra: &mut Option<(u8, Undo)>) {
    if extra.is_some() || !matches!(g.read(GAME_MODE), 0x16 | 0x0B) {
        return;
    }
    let level = g.read(LEVEL_NUMBER);
    if (level as usize) < extra_levels::FIRST {
        return;
    }
    if let Some((base, Some(undo))) = stand_in(g, level) {
        g.write(LEVEL_NUMBER, base);
        *extra = Some((level, undo));
    }
}

/// What a session is started with, besides the console itself.
pub struct SessionParts {
    pub prefs: Preferences,
    pub saves: Box<dyn SaveStore>,
    /// Go straight into this level (the level editor's Play button).
    pub start_level: Option<u8>,
    /// Unattended run for a check: no menu, faster, no hint.
    pub checking: bool,
}

pub struct GameSession {
    gb: Gb,
    prefs: Preferences,
    saves: Box<dyn SaveStore>,
    menu: Option<Menu>,
    checking: bool,
    acc: f64,
    save_timer: f32,
    /// After the menu closes, the game sees no buttons until every key is let go, so the
    /// key that closed it doesn't also press a button in the game.
    input_lock: bool,
    start_tap: u32,
    hint_time: f32,
    prev_mode: GameMode,
    screen: Vec<u8>,
    /// The new level being played (its number, and how to put its base level back once
    /// Mario is on the map again).
    extra: Option<(u8, Undo)>,
}

impl GameSession {
    pub fn new(mut gb: Gb, parts: SessionParts) -> GameSession {
        let mut extra = None;
        if let Some(n) = parts.start_level {
            if gb.boot_to_map() {
                if let Some((ask, undo)) = stand_in(&mut gb, n) {
                    extra = undo.map(|u| (n, u));
                    gb.enter_level(ask.min(31));
                }
            }
        }
        let screen = gb.screen().to_vec();
        GameSession {
            gb,
            prefs: parts.prefs,
            saves: parts.saves,
            menu: None,
            checking: parts.checking,
            acc: 0.0,
            save_timer: 0.0,
            input_lock: false,
            start_tap: 0,
            hint_time: if parts.checking { 0.0 } else { 4.0 },
            prev_mode: GameMode::Other(0),
            screen,
            extra,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.prefs.settings
    }

    /// What to show: the game's picture with the port's own drawing on top, in shades 0-3.
    pub fn screen(&self) -> &[u8] {
        &self.screen
    }

    pub fn palette(&self) -> &'static Palette {
        &PALETTES[self.prefs.settings.palette % PALETTES.len()]
    }

    /// Write the battery save (also done every few seconds while playing).
    pub fn finish(&mut self) {
        self.saves.save(&mut self.gb);
    }

    /// One window frame lasting `dt` seconds.
    pub fn frame(&mut self, dt: f32, input: &dyn Keyboard, audio: &dyn AudioOut) -> Flow {
        if input.pressed(Key::F11) {
            self.prefs.toggle_fullscreen();
        }
        let g = &mut self.gb;
        // In a level the Start key opens this pause menu; elsewhere it is the game's own
        // Start. Esc opens it anywhere.
        let mode = g.mode();
        let in_level = mode == GameMode::Playing;
        // Show the "ESC: menu / Enter: pause" tag again each time a level begins.
        if in_level && self.prev_mode != GameMode::Playing && self.prev_mode != GameMode::Paused && !self.checking {
            self.hint_time = 3.0;
        }
        self.prev_mode = mode;
        // back on the map: the base level of a new level is itself again
        if mode == GameMode::Map {
            if let Some((_, undo)) = self.extra.take() {
                extra_levels::apply(g.cart_mut().rom_mut(), &undo);
            }
        }

        if self.menu.is_none() {
            if !self.checking {
                let start_key = input.pressed(self.prefs.settings.keys[keys::START]) && in_level && !self.input_lock;
                if input.pressed(Key::Escape) || start_key {
                    self.menu = Some(Menu::new(mode));
                }
            }
        } else if let Some(m) = self.menu.as_mut() {
            let before = self.prefs.settings.clone();
            let action = m.update(input, &mut self.prefs.settings);
            self.prefs.changed_from(&before);
            match action {
                Action::None => {}
                Action::Close => self.close_menu(),
                Action::PressStart => {
                    self.close_menu();
                    self.start_tap = 8;
                }
                Action::Quit => return Flow::Quit,
                Action::BackToMap => {
                    self.close_menu();
                    self.gb.back_to_map();
                }
                Action::FileSelect => {
                    self.close_menu();
                    self.gb.back_to_file_select();
                    self.acc = 0.0;
                }
            }
        }

        if self.checking {
            // Checks: run the game a little faster so the title screen is up.
            for _ in 0..4 {
                self.gb.run_frame();
            }
            self.gb.clear_audio();
        } else if self.menu.is_some() {
            // Paused: the game is frozen and silent.
            self.gb.clear_audio();
            self.acc = 0.0;
        } else {
            self.play(dt, input, audio, in_level);
        }

        let st = &self.prefs.settings;
        self.screen.copy_from_slice(self.gb.screen());
        if let Some(m) = &self.menu {
            m.draw(&mut self.screen, st);
        } else if self.hint_time > 0.0 {
            self.hint_time -= dt;
            draw_hint(&mut self.screen, st.lang);
        }
        Flow::Continue
    }

    fn close_menu(&mut self) {
        self.menu = None;
        self.input_lock = true;
    }

    /// The game runs: buttons in, frames at the console's pace, sound out, periodic saves.
    fn play(&mut self, dt: f32, input: &dyn Keyboard, audio: &dyn AudioOut, in_level: bool) {
        let st = &self.prefs.settings;
        let g = &mut self.gb;
        let mut b = keys::buttons(input, &st.keys);
        let held_any = !b.is_empty() || input.held(Key::Enter) || input.held(Key::Space) || input.held(Key::Escape);
        if self.input_lock {
            if held_any {
                b = Buttons::NONE;
            } else {
                self.input_lock = false;
            }
        }
        if in_level {
            b &= !Buttons::START;
        }
        if self.start_tap > 0 {
            self.start_tap -= 1;
            b |= Buttons::START;
        }
        g.set_buttons(b);
        let fast = input.held(Key::LeftControl) || input.held(Key::RightControl);
        self.acc += dt.min(0.1) as f64;
        // Keep the sound queue fed: nudge timing when it runs low or high.
        let q = audio.queued_seconds();
        let target = FRAME_SECONDS * if q < 0.03 { 0.9 } else if q > 0.09 { 1.1 } else { 1.0 };
        let mut ran = 0;
        while self.acc >= target && ran < 4 {
            self.acc -= target;
            g.run_frame();
            take_new_level(g, &mut self.extra);
            ran += 1;
        }
        if fast {
            for _ in 0..3 {
                g.run_frame();
                take_new_level(g, &mut self.extra);
            }
            g.clear_audio();
        }
        audio.push(g.audio(), st.volume as f32 / 10.0);
        g.clear_audio();
        self.save_timer += dt;
        if self.save_timer > 5.0 {
            self.save_timer = 0.0;
            self.saves.save(g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{SettingsStore, Window};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Keys held during the current frame; a key counts as pressed on its first frame.
    #[derive(Default)]
    struct FakeKeys {
        now: Vec<Key>,
        before: Vec<Key>,
    }
    impl FakeKeys {
        fn set(&mut self, keys: &[Key]) {
            self.before = std::mem::replace(&mut self.now, keys.to_vec());
        }
    }
    impl Keyboard for FakeKeys {
        fn pressed(&self, k: Key) -> bool {
            self.now.contains(&k) && !self.before.contains(&k)
        }
        fn held(&self, k: Key) -> bool {
            self.now.contains(&k)
        }
        fn last_pressed(&self) -> Option<Key> {
            self.now.iter().copied().find(|k| !self.before.contains(k))
        }
    }

    struct Silence;
    impl AudioOut for Silence {
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn push(&self, _: &[f32], _: f32) {}
        fn queued_seconds(&self) -> f32 {
            0.05
        }
    }

    #[derive(Clone, Default)]
    struct Record {
        settings_saved: Rc<RefCell<Vec<Settings>>>,
        fullscreen: Rc<Cell<bool>>,
        battery_saves: Rc<Cell<u32>>,
    }
    impl SettingsStore for Record {
        fn load(&self) -> Settings {
            Settings::default()
        }
        fn save(&self, s: &Settings) {
            self.settings_saved.borrow_mut().push(s.clone());
        }
    }
    impl Window for Record {
        fn set_fullscreen(&mut self, on: bool) {
            self.fullscreen.set(on);
        }
    }
    impl SaveStore for Record {
        fn save(&self, _: &mut Gb) {
            self.battery_saves.set(self.battery_saves.get() + 1);
        }
    }

    /// Plays from power-on into the first level with keys only, then uses the pause menu.
    #[test]
    fn play_into_a_level_and_use_the_menu() {
        let Some(rom) = host::find_rom(None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let gb = host::load_gb(&rom, 44100, false).unwrap();
        let rec = Record::default();
        let prefs = Preferences::new(Box::new(rec.clone()), Box::new(rec.clone()));
        let parts = SessionParts {
            prefs,
            saves: Box::new(rec.clone()),
            start_level: None,
            checking: false,
        };
        let mut s = GameSession::new(gb, parts);
        let mut keys = FakeKeys::default();
        let frame = |s: &mut GameSession, keys: &mut FakeKeys, held: &[Key]| {
            keys.set(held);
            assert_eq!(s.frame(FRAME_SECONDS as f32, keys, &Silence), Flow::Continue);
        };

        // Title, file select, map: Start (Enter) now and then until a level is playing.
        let mut f = 0;
        while s.gb.mode() != GameMode::Playing {
            assert!(f < 3000, "never reached a level");
            let start = f % 150 >= 140;
            frame(&mut s, &mut keys, if start { &[Key::Enter] } else { &[] });
            f += 1;
        }
        assert!(rec.battery_saves.get() > 0, "a battery save every 5 seconds of play");

        // Esc opens the menu over a frozen game.
        frame(&mut s, &mut keys, &[]);
        frame(&mut s, &mut keys, &[Key::Escape]);
        let frozen = s.gb.screen().to_vec();
        for _ in 0..30 {
            frame(&mut s, &mut keys, &[]);
        }
        assert_eq!(s.gb.screen().to_vec(), frozen);
        assert_ne!(s.screen(), &frozen[..], "menu drawn over the picture");

        // Display → full screen on: saved and applied to the window.
        for k in [Key::Down, Key::Down, Key::Enter, Key::Down, Key::Enter] {
            frame(&mut s, &mut keys, &[]);
            frame(&mut s, &mut keys, &[k]);
        }
        assert!(s.settings().fullscreen);
        assert!(rec.fullscreen.get());
        assert!(rec.settings_saved.borrow().last().unwrap().fullscreen);

        // Back out and close: the game runs again.
        for k in [Key::Escape, Key::Escape] {
            frame(&mut s, &mut keys, &[]);
            frame(&mut s, &mut keys, &[k]);
        }
        // (the "ESC: MENU" tag shown when the level began stays for 3 seconds of play)
        for _ in 0..200 {
            frame(&mut s, &mut keys, &[]);
        }
        assert_eq!(s.screen(), s.gb.screen().as_slice());

        let before = rec.battery_saves.get();
        s.finish();
        assert_eq!(rec.battery_saves.get(), before + 1);
    }

    /// A new level (32) opened by a point of the game's own map (point $00, where a new game
    /// starts): the game's Start enters it, it is played in place of its base level, and the
    /// base level is back once on the map.
    #[test]
    fn a_new_level_is_played_from_the_original_map() {
        let Some(rom) = host::find_rom(None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        let mut e = extra_levels::based_on(&copy, 0);
        for c in 30..90 {
            e.map[30 * cart_model::levels::COLS + c] = 1;
        }
        extra_levels::store(&mut copy, &[e.clone()]).unwrap();
        let mut points = cart_model::world::point_levels(&copy);
        points[0] = 32;
        cart_model::world::write_point_levels(&mut copy, &points);
        let gb = Gb::new(copy.clone(), 44100).unwrap();
        let rec = Record::default();
        let parts = SessionParts {
            prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
            saves: Box::new(rec.clone()),
            start_level: None,
            checking: false,
        };
        let mut s = GameSession::new(gb, parts);
        let mut keys = FakeKeys::default();
        let mut f = 0;
        while s.gb.mode() != GameMode::Playing {
            assert!(f < 3000, "never reached a level");
            keys.set(if f % 150 >= 140 { &[Key::Enter] } else { &[] });
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            f += 1;
        }
        assert_eq!(s.extra.as_ref().map(|x| x.0), Some(32));
        assert_eq!(cart_model::live::map_in_memory(&s.gb), e.map, "the new level's map is played");
        keys.set(&[]);
        for _ in 0..30 {
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
        }
        s.gb.back_to_map();
        for _ in 0..600 {
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
        }
        assert_eq!(s.gb.mode(), GameMode::Map);
        assert!(s.extra.is_none() && s.gb.cart().rom() == &copy[..], "the base level is back");
    }
}
