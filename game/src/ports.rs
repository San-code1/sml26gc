//! What the game needs from the computer, as small traits. `main.rs` decides which
//! implementations (adapters) are plugged in: a real window and speakers, or silent ones
//! for checks and tests.
use crate::domain::keys::Key;
use crate::domain::settings::Settings;
use gb_core::Gb;

/// The keyboard as seen during one frame.
pub trait Keyboard {
    /// Went down this frame.
    fn pressed(&self, key: Key) -> bool;
    /// Is down now.
    fn held(&self, key: Key) -> bool;
    /// The last key that went down this frame, if the game knows it.
    fn last_pressed(&self) -> Option<Key>;
}

/// Where the sound goes.
pub trait AudioOut {
    fn sample_rate(&self) -> u32;
    /// Interleaved stereo samples.
    fn push(&self, samples: &[f32], volume: f32);
    /// How much sound is waiting to be played.
    fn queued_seconds(&self) -> f32;
}

/// The window, for what the game itself decides about it.
pub trait Window {
    fn set_fullscreen(&mut self, on: bool);
}

pub trait SettingsStore {
    fn load(&self) -> Settings;
    fn save(&self, settings: &Settings);
}

/// The cartridge's battery-backed memory (the game's own save slots).
pub trait SaveStore {
    fn save(&self, gb: &mut Gb);
}
