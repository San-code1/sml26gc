//! The player's settings together with what follows from changing them: they are saved,
//! and the window follows the full-screen choice.
use crate::domain::settings::Settings;
use crate::ports::{SettingsStore, Window};

pub struct Preferences {
    pub settings: Settings,
    store: Box<dyn SettingsStore>,
    window: Box<dyn Window>,
}

impl Preferences {
    pub fn new(store: Box<dyn SettingsStore>, mut window: Box<dyn Window>) -> Preferences {
        let settings = store.load();
        if settings.fullscreen {
            window.set_fullscreen(true);
        }
        Preferences { settings, store, window }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.settings.fullscreen = !self.settings.fullscreen;
        self.window.set_fullscreen(self.settings.fullscreen);
        self.store.save(&self.settings);
    }

    /// Call after something (the menu) edited `settings` directly.
    pub fn changed_from(&mut self, before: &Settings) {
        if *before == self.settings {
            return;
        }
        if before.fullscreen != self.settings.fullscreen {
            self.window.set_fullscreen(self.settings.fullscreen);
        }
        self.store.save(&self.settings);
    }
}
