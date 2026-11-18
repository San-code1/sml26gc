//! What the editor needs from the computer. `main.rs` plugs in the implementations.
use crate::domain::prefs::Prefs;
use std::path::{Path, PathBuf};

/// Where the edited copy is kept. The original cartridge is only ever read.
pub trait EditedCopyStore {
    fn path(&self) -> &Path;
    fn write(&self, rom: &[u8]) -> std::io::Result<()>;
    /// The project file, written before the cartridge (which is made from it).
    fn write_project(&self, text: &str) -> std::io::Result<()>;
    /// Where the editor was when the project was last saved (`place` text), if anywhere.
    fn read_place(&self) -> Option<String>;
    fn write_place(&self, text: &str) -> std::io::Result<()>;
}

/// The editor's own settings.
pub trait PrefsStore {
    fn load(&self) -> Prefs;
    fn save(&self, prefs: &Prefs);
}

/// Starting the game on the edited copy.
pub trait GameLauncher {
    /// A test game (no saves) with Mario's `lives` and `coins`; `level`: go straight into that
    /// level; `start`: with Mario there (his left x and feet y) this time only. The error names
    /// the program that failed.
    fn launch(&self, rom: &Path, level: Option<usize>, start: Option<(u16, u16)>, lives: u32, coins: u32) -> Result<(), String>;
}

/// Where the captured object pictures are kept between starts (`look_cache` text).
pub trait LookStore {
    fn load(&self) -> Option<String>;
    fn save(&self, text: &str);
}

/// The speakers, for listening to a level's music.
pub trait SoundOut {
    fn sample_rate(&self) -> u32;
    /// Interleaved stereo samples, at `volume` (0-1).
    fn push(&self, samples: &[f32], volume: f32);
    fn queued_seconds(&self) -> f32;
}

/// The kinds of file the editor exports and imports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileKind {
    Level,
    Zone,
    World,
    Project,
}

impl FileKind {
    pub fn extension(self) -> &'static str {
        match self {
            FileKind::Level => "nmblevel",
            FileKind::Zone => "nmbzone",
            FileKind::World => "nmbworld",
            FileKind::Project => "nmbproject",
        }
    }
}

/// Asking the person for a file. None if they cancel.
pub trait FileDialogs {
    /// Where to write a new file, `name` suggested.
    fn save_as(&self, kind: FileKind, name: &str) -> Option<PathBuf>;
    fn open(&self, kind: FileKind) -> Option<PathBuf>;
    /// A Game Boy cartridge file, for the base cartridge.
    fn pick_cartridge(&self) -> Option<PathBuf>;
    /// The language of the dialogs' titles from now on.
    fn set_lang(&mut self, lang: i18n::Lang);
}

pub trait TextFiles {
    fn read(&self, path: &Path) -> std::io::Result<String>;
    fn write(&self, path: &Path, text: &str) -> std::io::Result<()>;
}

pub enum MenuEntry<'a> {
    /// `keys`: its shortcut, shown at the right.
    Item { id: u16, label: &'a str, keys: &'a str },
    Separator,
}

/// The window's menu bar.
pub trait MenuBar {
    /// Add a menu at the right of the bar. Ids are not 0.
    fn add_menu(&mut self, title: &str, entries: &[MenuEntry]);
    /// The ids of the entries chosen since the last call.
    fn chosen(&mut self) -> Vec<u16>;
    fn enable(&mut self, id: u16, on: bool);
    /// Tick entry `id` or take its tick away.
    fn check(&mut self, id: u16, on: bool);
    /// Take every menu off the bar (to build it again).
    fn clear(&mut self);
}
