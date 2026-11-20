//! The player's settings together with what follows from changing them: they are saved,
//! and the window follows the full-screen choice.
use crate::domain::settings::Settings;
use crate::ports::{SettingsStore, Window};

pub struct Preferences {
    pub settings: Settings,
    store: Box<dyn SettingsStore>,
    window: Box<dyn Window>,
    /// A language for this game only (a test game from the editor): the one the player's
    /// settings keep, written back whenever they are saved.
    kept_lang: Option<(usize, bool)>,
}

impl Preferences {
    pub fn new(store: Box<dyn SettingsStore>, mut window: Box<dyn Window>) -> Preferences {
        let settings = store.load();
        if settings.fullscreen {
            window.set_fullscreen(true);
        }
        Preferences { settings, store, window, kept_lang: None }
    }

    /// Speak `lang` in this game only; the settings file keeps its own language.
    pub fn speak_for_this_game(&mut self, lang: usize) {
        self.kept_lang = Some((self.settings.lang, self.settings.lang_auto));
        self.settings.lang = lang;
    }

    fn save(&self) {
        let mut s = self.settings.clone();
        if let Some((lang, auto)) = self.kept_lang {
            (s.lang, s.lang_auto) = (lang, auto);
        }
        self.store.save(&s);
    }

    pub fn toggle_fullscreen(&mut self) {
        self.settings.fullscreen = !self.settings.fullscreen;
        self.window.set_fullscreen(self.settings.fullscreen);
        self.save();
    }

    /// Call after something (the menu) edited `settings` directly.
    pub fn changed_from(&mut self, before: &Settings) {
        if *before == self.settings {
            return;
        }
        if before.fullscreen != self.settings.fullscreen {
            self.window.set_fullscreen(self.settings.fullscreen);
        }
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Kept(Rc<RefCell<Settings>>);
    impl SettingsStore for Kept {
        fn load(&self) -> Settings {
            self.0.borrow().clone()
        }
        fn save(&self, s: &Settings) {
            *self.0.borrow_mut() = s.clone();
        }
    }
    struct NoWindow;
    impl Window for NoWindow {
        fn set_fullscreen(&mut self, _: bool) {}
    }

    #[test]
    fn a_language_for_this_game_only_is_not_saved() {
        let file = Rc::new(RefCell::new(Settings { lang: 1, lang_auto: false, ..Settings::default() }));
        let mut p = Preferences::new(Box::new(Kept(file.clone())), Box::new(NoWindow));
        p.speak_for_this_game(5);
        assert_eq!(p.settings.lang, 5, "the game speaks German");
        let before = p.settings.clone();
        p.settings.volume = 3;
        p.changed_from(&before);
        assert_eq!((file.borrow().lang, file.borrow().volume), (1, 3), "the file keeps Spanish, and the new volume");
    }
}

