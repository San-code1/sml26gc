//! What the editor needs from the computer. `main.rs` plugs in the implementations.
use crate::domain::prefs::Prefs;
use std::path::Path;

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
