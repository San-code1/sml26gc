//! What the game needs from the computer, as small traits. `main.rs` decides which
//! implementations (adapters) are plugged in: a real window and speakers, or silent ones
//! for checks and tests.
use crate::domain::keys::Key;
use crate::domain::pad::PadButton;
use crate::domain::settings::Settings;
use crate::domain::zone_progress::ZoneProgress;
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

/// The controller as seen during one frame (the first one connected).
pub trait Gamepad {
    fn connected(&self) -> bool;
    /// Went down this frame.
    fn pressed(&self, b: PadButton) -> bool;
    /// Is down now.
    fn held(&self, b: PadButton) -> bool;
    /// The last button that went down this frame.
    fn last_pressed(&self) -> Option<PadButton>;
}

/// No controller at all (tests).
#[cfg(test)]
pub struct NoPad;

#[cfg(test)]
impl Gamepad for NoPad {
    fn connected(&self) -> bool {
        false
    }
    fn pressed(&self, _: PadButton) -> bool {
        false
    }
    fn held(&self, _: PadButton) -> bool {
        false
    }
    fn last_pressed(&self) -> Option<PadButton> {
        None
    }
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

/// Progress in the editor's new zones, kept next to the battery save.
pub trait ZoneProgressStore {
    fn load(&self) -> Option<ZoneProgress>;
    fn save(&self, progress: &ZoneProgress);
}
