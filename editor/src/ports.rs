//! What the editor needs from the computer. `main.rs` plugs in the implementations.
use crate::domain::prefs::Prefs;
use std::path::{Path, PathBuf};

/// Where the edited copy is kept. The original cartridge is only ever read.
pub trait EditedCopyStore {
    fn path(&self) -> &Path;
    fn write(&self, rom: &[u8]) -> std::io::Result<()>;
}

/// The editor's own settings.
pub trait PrefsStore {
    fn load(&self) -> Prefs;
    fn save(&self, prefs: &Prefs);
}

/// Starting the game on the edited copy.
pub trait GameLauncher {
    /// `level`: go straight into that level. The error names the program that failed.
    fn launch(&self, rom: &Path, level: Option<usize>) -> Result<(), String>;
}

/// The kinds of file the editor exports and imports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileKind {
    Level,
    Zone,
    Project,
}

impl FileKind {
    pub fn extension(self) -> &'static str {
        match self {
            FileKind::Level => "nmblevel",
            FileKind::Zone => "nmbzone",
            FileKind::Project => "nmbproject",
        }
    }
}

/// Asking the person for a file. None if they cancel.
pub trait FileDialogs {
    /// Where to write a new file, `name` suggested.
    fn save_as(&self, kind: FileKind, name: &str) -> Option<PathBuf>;
    fn open(&self, kind: FileKind) -> Option<PathBuf>;
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
}
