//! What the editor needs from the computer. `main.rs` plugs in the implementations.
use cart_model::custom_map::CustomMap;
use std::path::Path;

/// Where the edited copy (and its editor-made world map) is kept. The original cartridge
/// is only ever read.
pub trait EditedCopyStore {
    fn path(&self) -> &Path;
    fn write(&self, rom: &[u8]) -> std::io::Result<()>;
    /// A map with no points removes the map file (the game then uses its own).
    fn save_custom_map(&self, map: &CustomMap) -> std::io::Result<()>;
}

/// Starting the game on the edited copy.
pub trait GameLauncher {
    /// `level`: go straight into that level. The error names the program that failed.
    fn launch(&self, rom: &Path, level: Option<usize>) -> Result<(), String>;
}
