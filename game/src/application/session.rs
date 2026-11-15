//! One play session: the emulated console plus everything the port adds around it (pause
//! menu, editor-made world map, cleared levels, battery saves, the start-up hint). Each
//! call to `frame` is one frame of the window.
use super::preferences::Preferences;
use crate::domain::keys::{self, Key};
use crate::domain::map_view::MapView;
use crate::domain::menu::{Action, Menu};
use crate::domain::overlay::draw_hint;
use crate::domain::progress::LevelTracker;
use crate::domain::settings::Settings;
use crate::ports::{AudioOut, Keyboard, ProgressStore, SaveStore};
use cart_model::ram_map::LEVEL_NUMBER;
use cart_model::{GameDriver, GameMode};
use gb_core::palette::{Palette, PALETTES};
use gb_core::{Buttons, Gb, FRAME_SECONDS};

#[derive(PartialEq, Debug)]
pub enum Flow {
    Continue,
    Quit,
}

/// What a session is started with, besides the console itself.
pub struct SessionParts {
    pub prefs: Preferences,
    pub saves: Box<dyn SaveStore>,
    pub progress: Box<dyn ProgressStore>,
    /// The editor-made world map for this cartridge, if there is one.
    pub map_view: Option<MapView>,
    /// Go straight into this level (the level editor's Play button).
    pub start_level: Option<u8>,
    /// Unattended run for a check: no menu, faster, no hint.
    pub checking: bool,
}

pub struct GameSession {
    gb: Gb,
    prefs: Preferences,
    saves: Box<dyn SaveStore>,
    progress: Box<dyn ProgressStore>,
    map_view: Option<MapView>,
    tracker: LevelTracker,
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
    map_frames: u32,
    screen: Vec<u8>,
}

impl GameSession {
    pub fn new(mut gb: Gb, parts: SessionParts) -> GameSession {
        if let Some(n) = parts.start_level {
            if gb.boot_to_map() {
                gb.enter_level(n.min(31));
            }
        }
        let screen = gb.screen().to_vec();
        GameSession {
            gb,
            prefs: parts.prefs,
            saves: parts.saves,
            progress: parts.progress,
            map_view: parts.map_view,
            tracker: LevelTracker::default(),
            menu: None,
            checking: parts.checking,
            acc: 0.0,
            save_timer: 0.0,
            input_lock: false,
            start_tap: 0,
            hint_time: if parts.checking { 0.0 } else { 4.0 },
            prev_mode: GameMode::Other(0),
            map_frames: 0,
            screen,
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
        if let Some(level) = self.tracker.observe(mode, || g.read(LEVEL_NUMBER)) {
            if let Some(v) = self.map_view.as_mut() {
                if v.mark_cleared(level) {
                    self.progress.save(&v.cleared);
                }
            }
        }
        self.map_frames = if mode == GameMode::Map { self.map_frames + 1 } else { 0 };
        // The editor-made map takes over the world map screen once it has finished appearing.
        let custom = mode == GameMode::Map && self.map_frames > 20 && self.map_view.as_ref().map(|v| v.enabled).unwrap_or(false);

        if self.menu.is_none() {
            if !self.checking {
                let start_key = input.pressed(self.prefs.settings.keys[keys::START]) && in_level && !self.input_lock;
                if input.pressed(Key::Escape) || start_key {
                    self.menu = Some(Menu::new(mode, self.map_view.as_ref().map(|v| v.enabled)));
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
                    self.tracker.abandon();
                    self.gb.back_to_map();
                }
                Action::ToggleWorld => {
                    if let Some(v) = self.map_view.as_mut() {
                        v.enabled = !v.enabled;
                    }
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
            self.play(dt, input, audio, in_level, custom);
        }

        let st = &self.prefs.settings;
        self.screen.copy_from_slice(self.gb.screen());
        if custom {
            if let Some(v) = &self.map_view {
                v.draw(&mut self.screen, st.lang);
            }
        }
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
    fn play(&mut self, dt: f32, input: &dyn Keyboard, audio: &dyn AudioOut, in_level: bool, custom: bool) {
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
        if custom {
            // Our map gets the buttons; the game's own map idles underneath (its music plays).
            let v = self.map_view.as_mut().unwrap();
            v.prepare(g);
            let lock = self.input_lock;
            let k = |i: usize| input.held(st.keys[i]) && !lock;
            let enter = !lock && (input.pressed(st.keys[keys::A]) || input.pressed(st.keys[keys::START]));
            if let Some(level) = v.update(k(keys::LEFT), k(keys::RIGHT), k(keys::UP), k(keys::DOWN), enter) {
                g.enter_level_anywhere(level);
                self.acc = 0.0;
                self.input_lock = true;
            }
            v.prepare(g); // through a door: the new picture is drawn this frame already
            b = Buttons::NONE;
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
            ran += 1;
        }
        if fast {
            for _ in 0..3 {
                g.run_frame();
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
    use std::collections::HashSet;
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
    impl ProgressStore for Record {
        fn save(&self, _: &HashSet<u8>) {}
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
            progress: Box::new(rec.clone()),
            map_view: None,
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
}
