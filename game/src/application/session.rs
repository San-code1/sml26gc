//! One play session: the emulated console plus everything the port adds around it (pause
//! menu, the editor's new levels and zones, battery saves, the start-up hint). Each call to `frame`
//! is one frame of the window.
use super::preferences::Preferences;
use super::zones::ZoneSwap;
use crate::domain::keys::{self, Key};
use crate::domain::menu::{Action, Menu};
use crate::domain::overlay::draw_hint;
use crate::domain::settings::Settings;
use crate::ports::{AudioOut, Keyboard, SaveStore, ZoneProgressStore};
use cart_model::extra_levels;
use cart_model::secret_exits;
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
/// `extra_levels`) is put in place of its base level in the console's ROM (its secret exit's
/// target too), and the writes that undo it come back. None if the copy has no such level.
fn stand_in(g: &mut Gb, level: u8) -> Option<(u8, Option<Undo>)> {
    let Some(k) = (level as usize).checked_sub(extra_levels::FIRST) else { return Some((level, None)) };
    let e = extra_levels::read(g.cart().rom()).into_iter().nth(k)?;
    let mut writes = extra_levels::writes_for(g.cart().rom(), &e);
    writes.extend(secret_exits::stand_in(g.cart().rom(), level as usize, e.base as usize));
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
    pub zones: Box<dyn ZoneProgressStore>,
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
    zones: ZoneSwap,
}

impl GameSession {
    pub fn new(mut gb: Gb, parts: SessionParts) -> GameSession {
        let zones = ZoneSwap::new(&mut gb, parts.zones);
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
            zones,
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
        b |= self.zones.buttons();
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
        if self.zones.update(g) || self.save_timer > 5.0 {
            self.save_timer = 0.0;
            self.saves.save(g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::zone_progress::{MapKey, ZoneProgress};
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
        zones: Rc<RefCell<Option<ZoneProgress>>>,
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
    impl ZoneProgressStore for Record {
        fn load(&self) -> Option<ZoneProgress> {
            self.zones.borrow().clone()
        }
        fn save(&self, p: &ZoneProgress) {
            *self.zones.borrow_mut() = Some(p.clone());
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
            zones: Box::new(rec.clone()),
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
            zones: Box::new(rec.clone()),
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

    /// A new level's secret exit sent into the Pumpkin Zone: played from the level editor's
    /// Play and from a map point that opens it, Mario comes out on the Pumpkin Zone's $28 and
    /// stays there.
    #[test]
    fn a_new_level_secret_exit_leads_where_chosen() {
        use cart_model::secret_exits::{self, SecretTo};
        let Some(rom) = host::find_rom(None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        let e = extra_levels::based_on(&copy, 0);
        extra_levels::store(&mut copy, &[e]).unwrap();
        let mut points = cart_model::world::point_levels(&copy);
        points[0] = 32;
        cart_model::world::write_point_levels(&mut copy, &points);
        secret_exits::write(&mut copy, &[(32, SecretTo::Point(0x28))], &[]).unwrap();
        let rec = Record::default();
        let session = |start_level| {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level,
                checking: false,
            };
            GameSession::new(Gb::new(copy.clone(), 44100).unwrap(), parts)
        };
        let mut keys = FakeKeys::default();
        for start_level in [Some(32), None] {
            let mut s = session(start_level);
            let mut f = 0;
            while s.gb.mode() != GameMode::Playing {
                assert!(f < 3000, "never reached a level");
                keys.set(if f % 150 >= 140 { &[Key::Enter] } else { &[] });
                s.frame(FRAME_SECONDS as f32, &keys, &Silence);
                f += 1;
            }
            keys.set(&[]);
            assert_eq!(s.extra.as_ref().map(|x| x.0), Some(32));
            s.gb.write(0xA2B4, 0xFF);
            assert!(clear_level(&mut s.gb));
            for _ in 0..300 {
                s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            }
            assert_eq!((s.gb.read(cart_model::ram_map::MAP_POINT), s.gb.read(0xA68B)), (0x28, 1), "{start_level:?}: on the Pumpkin Zone's $28, staying there");
        }
    }

    /// Clear the level of the point Mario stands on as at a goal (mode $09), then play on
    /// until the map has been up for a while.
    fn clear_level(g: &mut Gb) -> bool {
        let mut playing = false;
        for f in 0..900u32 {
            g.run_frame_with(if f % 30 < 4 { Buttons::START } else { Buttons::NONE });
            if g.mode() == GameMode::Playing {
                playing = true;
                break;
            }
        }
        for _ in 0..30 {
            g.run_frame_with(Buttons::NONE);
        }
        g.write(GAME_MODE, 0x09);
        let mut on_map = 0;
        for f in 0..6000u32 {
            let mode = g.read(GAME_MODE);
            let tap = match f % 240 {
                0..=2 => Buttons::A,
                120..=122 => Buttons::START,
                _ => Buttons::NONE,
            };
            g.run_frame_with(if mode != 0x0C && mode != 0x14 { tap } else { Buttons::NONE });
            on_map = if g.read(GAME_MODE) == 0x0C { on_map + 1 } else { 0 };
            if on_map > 300 {
                break;
            }
        }
        g.clear_audio();
        playing
    }

    /// A new zone made from the Tree Zone, entered from world point $07: its own picture and
    /// a level cleared with its own flag; saved inside and resumed there; left by its exit,
    /// after which the cartridge and the Tree Zone's flags are the game's again; then entered
    /// by the Tree Zone's exit.
    #[test]
    fn a_new_zone_is_played_saved_and_left() {
        use cart_model::new_zones;
        use cart_model::ram_map::MAP_POINT;
        let Some(rom) = host::find_rom(None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        let mut z = new_zones::based_on(&copy, 0).unwrap();
        for t in z.picture.iter_mut().take(60) {
            *t ^= 1;
        }
        z.points.retain(|p| p.number == 0x23);
        z.points[0].ways = Default::default();
        z.exits[0].1 = cart_model::doors::Leads::World(0x07);
        z.door = 0x07;
        new_zones::store(&mut copy, std::slice::from_ref(&z)).unwrap();
        // the Tree Zone's own exit leads into the new zone (to its door)
        let mut d = cart_model::doors::read(&copy);
        d.exits[0].1 = cart_model::doors::Leads::World(0x07);
        assert!(cart_model::doors::write(&mut copy, &d));
        let rec = Record::default();
        let session = |gb: Gb| {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level: None,
                checking: false,
            };
            GameSession::new(gb, parts)
        };
        let run = |s: &mut GameSession, keys: &mut FakeKeys, held: &[Key], n: u32| {
            for f in 0..n {
                keys.set(if f < 6 { held } else { &[] });
                s.frame(FRAME_SECONDS as f32, keys, &Silence);
            }
        };
        let picture = |g: &mut Gb| (0..360).map(|i| g.read(0x9800 + ((i / 20) * 32 + i % 20) as u16)).collect::<Vec<u8>>();
        let mut keys = FakeKeys::default();

        let mut s = session(Gb::new(copy.clone(), 44100).unwrap());
        assert!(s.gb.boot_to_map() && s.gb.show_map_at(0x07));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.zones.inside(), Some(0), "at the door the new zone is in place");
        run(&mut s, &mut keys, &[Key::X], 400);
        assert_eq!(s.gb.read(MAP_POINT), 0x23);
        assert_eq!(picture(&mut s.gb), z.picture, "its own picture");
        assert!(clear_level(&mut s.gb));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.gb.read(0xA849), 0x80, "its point is cleared");
        s.finish();

        // resumed: the battery save and the kept progress put Mario back in the new zone
        let ram = s.gb.cart().ram().to_vec();
        let mut gb = Gb::new(copy.clone(), 44100).unwrap();
        gb.cart_mut().set_ram(ram);
        let mut s = session(gb);
        assert_eq!(s.zones.inside(), Some(0));
        assert!(s.gb.boot_to_map());
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.gb.read(MAP_POINT), 0x23);
        assert_eq!(picture(&mut s.gb), z.picture);
        assert_eq!(s.gb.read(0xA849), 0x80);

        // out by its exit to the door, then away from it: the game's own again
        run(&mut s, &mut keys, &[Key::Down], 600);
        assert_eq!(s.gb.read(MAP_POINT), 0x07);
        assert!(s.gb.show_map_at(0x02));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.zones.inside(), None);
        assert!(s.gb.cart().rom() == &copy[..], "the Tree Zone is back");
        assert_eq!(s.gb.read(0xA849), 0x00, "the Tree Zone's flag is its own");
        assert_eq!(rec.zones.borrow().as_ref().unwrap().kept[&(0, 0)][0], 0x80, "the new zone's progress is kept");

        // out of the Tree Zone into the new zone: the port presses A on its door
        run(&mut s, &mut keys, &[Key::X], 400);
        assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (0x23, None), "in the Tree Zone");
        run(&mut s, &mut keys, &[Key::Down], 900);
        assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (0x23, Some(0)));
        assert_eq!(picture(&mut s.gb), z.picture, "in the new zone");

        // level 8's secret exit leads into the new zone: Mario comes back on its door and the
        // port enters it
        use cart_model::secret_exits::{self, SecretTo};
        secret_exits::write(&mut copy, &[(8, SecretTo::NewZone(0))], &[0x07]).unwrap();
        let mut s = session(Gb::new(copy.clone(), 44100).unwrap());
        assert!(s.gb.boot_to_map() && s.gb.show_map_at(0x2A) && s.gb.enter_level(8));
        run(&mut s, &mut keys, &[], 30);
        s.gb.write(0xA2B4, 0xFF);
        assert!(clear_level(&mut s.gb));
        assert_eq!(s.gb.read(MAP_POINT), 0x07, "on the new zone's door");
        run(&mut s, &mut keys, &[], 400);
        assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (0x23, Some(0)), "in the new zone");
    }

    /// Added points ($80) on the world map (beside $07) and in the Tree Zone (beside $20):
    /// walked to and cleared with their own flag bytes, kept per map; a game saved on the
    /// Tree Zone's resumes there with it cleared, and the world map's stays apart.
    #[test]
    fn added_points_are_played_kept_and_resumed() {
        use cart_model::added_points::{self, ZoneAdded};
        use cart_model::map_routes::{self, Place, Way};
        use cart_model::new_zones::ZonePoint;
        use cart_model::ram_map::MAP_POINT;
        let Some(rom) = host::find_rom(None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        assert!(added_points::install(&mut copy));
        let p = added_points::FIRST;
        let step = [(24, 0), (-24, 0), (0, -24), (0, 24)];
        let keys_of = [Key::Right, Key::Left, Key::Up, Key::Down];
        let back = [1, 0, 3, 2];
        let mut r = map_routes::read(&copy);
        let free = |r: &map_routes::MapRoutes, q: u8| (0..4).find(|&d| r.ways[q as usize][d].is_none()).unwrap();
        // the world map's: in the routes as written
        let (dw, dz) = (free(&r, 0x07), free(&r, 0x20));
        let (x, y) = r.places[0x07].on_world();
        r.places[p as usize] = Place::at_world(x + step[dw].0, y + step[dw].1);
        r.ways[0x07][dw] = Some(Way::through(&[step[dw]], p));
        r.ways[p as usize][back[dw]] = Some(Way::through(&[(-step[dw].0, -step[dw].1)], 0x07));
        r.added.push(p);
        // the Tree Zone's: the way to it in the routes, the point kept for the port
        let at = r.places[0x20];
        r.ways[0x20][dz] = Some(Way::through(&[step[dz]], p));
        map_routes::write(&mut copy, &r).unwrap();
        let mut levels = cart_model::world::point_levels(&copy);
        levels[p as usize] = 1;
        cart_model::world::write_point_levels(&mut copy, &levels);
        let place = Place { x: (at.x as i32 + step[dz].0) as u8, y: (at.y as i32 + step[dz].1) as u8, ..at };
        let mut ways: [Option<Way>; 4] = Default::default();
        ways[back[dz]] = Some(Way::through(&[(-step[dz].0, -step[dz].1)], 0x20));
        let tree = ZonePoint { number: p, place, ways, level: 2, flag: p, shut: 0, pass: None };
        added_points::store_zones(&mut copy, &[ZoneAdded { zone: 0, points: vec![tree] }]).unwrap();

        let rec = Record::default();
        let session = |gb: Gb| {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level: None,
                checking: false,
            };
            GameSession::new(gb, parts)
        };
        let run = |s: &mut GameSession, keys: &mut FakeKeys, held: &[Key], n: u32| {
            for f in 0..n {
                keys.set(if f < 6 { held } else { &[] });
                s.frame(FRAME_SECONDS as f32, keys, &Silence);
            }
        };
        let own = |g: &mut Gb| g.read(added_points::FLAG_RAM);
        let mut keys = FakeKeys::default();

        let mut s = session(Gb::new(copy.clone(), 44100).unwrap());
        assert!(s.gb.boot_to_map() && s.gb.show_map_at(0x20));
        run(&mut s, &mut keys, &[], 10);
        run(&mut s, &mut keys, &[keys_of[dz]], 300);
        assert_eq!(s.gb.read(MAP_POINT), p, "walked to the Tree Zone's added point");
        let flags: Vec<u8> = (0xA848..0xA872).map(|a| s.gb.read(a)).collect();
        assert!(clear_level(&mut s.gb));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!((s.gb.read(MAP_POINT), own(&mut s.gb)), (p, 0x80), "back on it, cleared");
        assert_eq!((0xA848..0xA872).map(|a| s.gb.read(a)).collect::<Vec<u8>>(), flags, "no flag of the game changed");
        s.finish();

        // resumed on the Tree Zone's added point, with it cleared
        let ram = s.gb.cart().ram().to_vec();
        let mut gb = Gb::new(copy.clone(), 44100).unwrap();
        gb.cart_mut().set_ram(ram);
        let mut s = session(gb);
        let mut f = 0;
        while s.gb.mode() != GameMode::Map {
            assert!(f < 3000, "never reached the map");
            run(&mut s, &mut keys, if f % 150 == 140 { &[Key::Enter] } else { &[] }, 1);
            f += 1;
        }
        run(&mut s, &mut keys, &[], 10);
        assert_eq!((s.gb.read(MAP_POINT), s.gb.read(0xA68B), own(&mut s.gb)), (p, 2, 0x80), "in the Tree Zone");
        run(&mut s, &mut keys, &[keys_of[back[dz]]], 300);
        assert_eq!(s.gb.read(MAP_POINT), 0x20);

        // the world map's added point: its own flag, not cleared
        assert!(s.gb.show_map_at(0x07));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(own(&mut s.gb), 0, "the world map's added point is not cleared");
        run(&mut s, &mut keys, &[keys_of[dw]], 300);
        assert_eq!(s.gb.read(MAP_POINT), p, "walked to the world map's added point");
        assert!(s.gb.cart().rom() == &copy[..], "the world map's added points are the copy's");
        assert_eq!(rec.zones.borrow().as_ref().unwrap().added[&(MapKey::Zone(0), 0)][0], 0x80, "the Tree Zone's progress is kept");
    }
}
